use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use codex_protocol::ThreadId;
use codex_rollout::RolloutItem;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    AppendThreadItemsParams, ArchiveThreadParams, CreateThreadParams, DeleteThreadParams,
    InMemoryThreadStore, ListThreadsParams, LoadThreadHistoryParams, PersistContext,
    ReadThreadByRolloutPathParams, ReadThreadParams, ResumeThreadParams, StoredModelContext,
    StoredThread, StoredThreadHistory, ThreadPage, ThreadStore, ThreadStoreError,
    ThreadStoreFuture, ThreadStoreResult, UpdateThreadMetadataParams,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteThreadStoreConfig {
    pub url: String,
    pub token_env: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteSnapshot {
    create: CreateThreadParams,
    history: StoredThreadHistory,
}

pub struct RemoteThreadStore {
    inner: InMemoryThreadStore,
    client: reqwest::Client,
    base_url: String,
    token: String,
    loaded: Mutex<HashSet<ThreadId>>,
    creates: Mutex<HashMap<ThreadId, CreateThreadParams>>,
    sync_lock: Mutex<()>,
}

impl RemoteThreadStore {
    pub fn new(config: RemoteThreadStoreConfig) -> ThreadStoreResult<Self> {
        let token =
            std::env::var(&config.token_env).map_err(|_| ThreadStoreError::InvalidRequest {
                message: format!(
                    "remote thread store token env {} is not set",
                    config.token_env
                ),
            })?;
        if token.trim().is_empty() {
            return Err(ThreadStoreError::InvalidRequest {
                message: "remote thread store token is empty".to_string(),
            });
        }
        Ok(Self {
            inner: InMemoryThreadStore::default(),
            client: reqwest::Client::new(),
            base_url: config.url.trim_end_matches('/').to_string(),
            token,
            loaded: Mutex::new(HashSet::new()),
            creates: Mutex::new(HashMap::new()),
            sync_lock: Mutex::new(()),
        })
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
    }

    fn internal(error: impl std::fmt::Display) -> ThreadStoreError {
        ThreadStoreError::Internal {
            message: format!("remote thread store: {error}"),
        }
    }

    async fn fetch(&self, thread_id: ThreadId) -> ThreadStoreResult<RemoteSnapshot> {
        let response = self
            .request(reqwest::Method::GET, &format!("/threads/{thread_id}"))
            .send()
            .await
            .map_err(Self::internal)?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ThreadStoreError::ThreadNotFound { thread_id });
        }
        response
            .error_for_status()
            .map_err(Self::internal)?
            .json()
            .await
            .map_err(Self::internal)
    }

    async fn ensure_loaded(&self, thread_id: ThreadId) -> ThreadStoreResult<()> {
        if self.loaded.lock().await.contains(&thread_id) {
            return Ok(());
        }
        let snapshot = self.fetch(thread_id).await?;
        <InMemoryThreadStore as ThreadStore>::create_thread(&self.inner, snapshot.create.clone())
            .await?;
        <InMemoryThreadStore as ThreadStore>::resume_thread(
            &self.inner,
            ResumeThreadParams {
                thread_id,
                rollout_path: None,
                history: Some(Arc::new(snapshot.history.items)),
                history_revision: None,
                include_archived: true,
                metadata: snapshot.create.metadata.clone(),
            },
        )
        .await?;
        self.creates.lock().await.insert(thread_id, snapshot.create);
        self.loaded.lock().await.insert(thread_id);
        Ok(())
    }

    async fn sync(&self, thread_id: ThreadId) -> ThreadStoreResult<()> {
        // A late response from an older PUT must never overwrite a newer snapshot.
        let _sync_guard = self.sync_lock.lock().await;
        let create = self
            .creates
            .lock()
            .await
            .get(&thread_id)
            .cloned()
            .ok_or(ThreadStoreError::ThreadNotFound { thread_id })?;
        let history = <InMemoryThreadStore as ThreadStore>::load_history(
            &self.inner,
            LoadThreadHistoryParams {
                thread_id,
                include_archived: true,
            },
        )
        .await?;
        self.request(reqwest::Method::PUT, &format!("/threads/{thread_id}"))
            .json(&RemoteSnapshot { create, history })
            .send()
            .await
            .map_err(Self::internal)?
            .error_for_status()
            .map_err(Self::internal)?;
        Ok(())
    }
}

impl ThreadStore for RemoteThreadStore {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn create_thread(&self, params: CreateThreadParams) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            let thread_id = params.thread_id;
            if params
                .extra_config
                .as_ref()
                .and_then(|v| v.business_session_id.as_ref())
                .is_none()
            {
                return Err(ThreadStoreError::InvalidRequest {
                    message: "businessSessionId is required for remote persistence".to_string(),
                });
            }
            <InMemoryThreadStore as ThreadStore>::create_thread(&self.inner, params.clone())
                .await?;
            self.creates.lock().await.insert(thread_id, params);
            self.loaded.lock().await.insert(thread_id);
            self.sync(thread_id).await
        })
    }

    fn resume_thread(
        &self,
        params: ResumeThreadParams,
    ) -> ThreadStoreFuture<'_, Arc<Vec<RolloutItem>>> {
        Box::pin(async move {
            self.ensure_loaded(params.thread_id).await?;
            <InMemoryThreadStore as ThreadStore>::resume_thread(&self.inner, params).await
        })
    }

    fn append_items(&self, params: AppendThreadItemsParams) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            let thread_id = params.thread_id;
            self.ensure_loaded(thread_id).await?;
            <InMemoryThreadStore as ThreadStore>::append_items(&self.inner, params).await?;
            self.sync(thread_id).await
        })
    }

    fn persist_thread(
        &self,
        thread_id: ThreadId,
        context: PersistContext,
    ) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            self.ensure_loaded(thread_id).await?;
            <InMemoryThreadStore as ThreadStore>::persist_thread(&self.inner, thread_id, context)
                .await?;
            self.sync(thread_id).await
        })
    }

    fn flush_thread(&self, thread_id: ThreadId) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            self.ensure_loaded(thread_id).await?;
            self.sync(thread_id).await
        })
    }

    fn shutdown_thread(&self, thread_id: ThreadId) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            self.ensure_loaded(thread_id).await?;
            self.sync(thread_id).await
        })
    }

    fn discard_thread(&self, thread_id: ThreadId) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            self.loaded.lock().await.remove(&thread_id);
            self.creates.lock().await.remove(&thread_id);
            <InMemoryThreadStore as ThreadStore>::discard_thread(&self.inner, thread_id).await
        })
    }

    fn load_history(
        &self,
        params: LoadThreadHistoryParams,
    ) -> ThreadStoreFuture<'_, StoredThreadHistory> {
        Box::pin(async move {
            self.ensure_loaded(params.thread_id).await?;
            <InMemoryThreadStore as ThreadStore>::load_history(&self.inner, params).await
        })
    }

    fn load_latest_model_context(
        &self,
        params: LoadThreadHistoryParams,
    ) -> ThreadStoreFuture<'_, StoredModelContext> {
        Box::pin(async move {
            self.ensure_loaded(params.thread_id).await?;
            <InMemoryThreadStore as ThreadStore>::load_latest_model_context(&self.inner, params)
                .await
        })
    }

    fn read_thread(&self, params: ReadThreadParams) -> ThreadStoreFuture<'_, StoredThread> {
        Box::pin(async move {
            self.ensure_loaded(params.thread_id).await?;
            <InMemoryThreadStore as ThreadStore>::read_thread(&self.inner, params).await
        })
    }

    fn read_thread_by_rollout_path(
        &self,
        _params: ReadThreadByRolloutPathParams,
    ) -> ThreadStoreFuture<'_, StoredThread> {
        Box::pin(async {
            Err(ThreadStoreError::Unsupported {
                operation: "read_thread_by_rollout_path",
            })
        })
    }

    fn list_threads(&self, params: ListThreadsParams) -> ThreadStoreFuture<'_, ThreadPage> {
        Box::pin(async move {
            <InMemoryThreadStore as ThreadStore>::list_threads(&self.inner, params).await
        })
    }

    fn update_thread_metadata(
        &self,
        params: UpdateThreadMetadataParams,
    ) -> ThreadStoreFuture<'_, Option<StoredThread>> {
        Box::pin(async move {
            self.ensure_loaded(params.thread_id).await?;
            <InMemoryThreadStore as ThreadStore>::update_thread_metadata(&self.inner, params).await
        })
    }

    fn archive_thread(&self, params: ArchiveThreadParams) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            self.ensure_loaded(params.thread_id).await?;
            self.request(
                reqwest::Method::POST,
                &format!("/threads/{}/archive", params.thread_id),
            )
            .send()
            .await
            .map_err(Self::internal)?
            .error_for_status()
            .map_err(Self::internal)?;
            <InMemoryThreadStore as ThreadStore>::archive_thread(&self.inner, params).await
        })
    }

    fn unarchive_thread(&self, params: ArchiveThreadParams) -> ThreadStoreFuture<'_, StoredThread> {
        Box::pin(async move {
            self.ensure_loaded(params.thread_id).await?;
            <InMemoryThreadStore as ThreadStore>::unarchive_thread(&self.inner, params).await
        })
    }

    fn delete_thread(&self, params: DeleteThreadParams) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            let thread_id = params.thread_id;
            let response = self
                .request(reqwest::Method::DELETE, &format!("/threads/{thread_id}"))
                .send()
                .await
                .map_err(Self::internal)?;
            if response.status() != reqwest::StatusCode::NOT_FOUND {
                response.error_for_status().map_err(Self::internal)?;
            }
            self.loaded.lock().await.remove(&thread_id);
            self.creates.lock().await.remove(&thread_id);
            match <InMemoryThreadStore as ThreadStore>::delete_thread(&self.inner, params).await {
                Ok(()) | Err(ThreadStoreError::ThreadNotFound { .. }) => Ok(()),
                Err(error) => Err(error),
            }
        })
    }
}
