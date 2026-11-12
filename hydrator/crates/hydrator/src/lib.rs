//! Fetch and normalize a public federated profile, its posts, graph and thread.
//!
//! The library has two sources behind one shape: a Mastodon REST origin and a generic
//! ActivityPub origin. The web host can link it directly; [`nodus-hydrate`] exposes the
//! same calls over HTTP.

pub mod mastodon;
pub mod types;

pub use types::{Account, Emoji, Media, Page, Post};

use nodus_activitypub_sdk::ActivityPub;

/// Why a hydration read failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No source could answer.
    #[error("not available: {0}")]
    Unavailable(String),
    /// The remote host failed.
    #[error("remote read failed: {0}")]
    Remote(String),
}

/// Read public federation data, preferring ActivityPub when a handle is known.
#[derive(Debug, Clone, Default)]
pub struct Hydrator {
    ap: ActivityPub,
    http: reqwest::Client,
}

impl Hydrator {
    /// A hydrator with the default HTTP client.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ap: ActivityPub::new(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("nodus-hydrator/0.1")
                .build()
                .unwrap_or_default(),
        }
    }

    /// Read a profile by handle; this is the ActivityPub path.
    ///
    /// # Errors
    /// [`Error::Remote`] when the actor cannot be read.
    pub async fn account_acct(&self, acct: &str) -> Result<Account, Error> {
        let actor = self.ap.actor(acct).await.map_err(remote)?;
        let host = acct.split_once('@').map_or("", |(_, host)| host);
        let followers = match actor.followers.as_deref() {
            Some(url) => self
                .ap
                .collection(url)
                .await
                .map(|collection| collection.total_items)
                .unwrap_or_default(),
            None => 0,
        };
        let following = match actor.following.as_deref() {
            Some(url) => self
                .ap
                .collection(url)
                .await
                .map(|collection| collection.total_items)
                .unwrap_or_default(),
            None => 0,
        };
        Ok(Account {
            host: host.to_owned(),
            id: actor.id.clone(),
            acct: actor.acct(host),
            display_name: actor.name,
            note: actor.summary,
            avatar: actor.avatar,
            header: actor.header,
            followers_count: followers,
            following_count: following,
            url: actor.url,
        })
    }

    /// Read a profile by Mastodon host and id.
    ///
    /// # Errors
    /// [`Error::Remote`] when the instance does not answer with an account.
    pub async fn account(&self, host: &str, id: &str) -> Result<Account, Error> {
        let value = self.get(host, &format!("/api/v1/accounts/{id}")).await?;
        Ok(mastodon::account(host, &value))
    }

    /// Read a page of an ActivityPub outbox.
    ///
    /// # Errors
    /// [`Error::Remote`] when the outbox cannot be read.
    pub async fn posts_acct(&self, acct: &str, page: Option<&str>) -> Result<Page<Post>, Error> {
        let actor = self.ap.actor(acct).await.map_err(remote)?;
        let collection = match page {
            Some(url) => self.ap.page(url).await.map_err(remote)?,
            None => self.ap.outbox(&actor).await.map_err(remote)?,
        };
        let mut posts = Vec::new();
        for item in collection.items {
            if let Ok(note) = nodus_activitypub_sdk::Note::from_json(&item) {
                posts.push(ap_post(note));
            }
        }
        Ok(Page {
            items: posts,
            next: collection.next,
        })
    }

    /// Read a page of a Mastodon account's posts.
    ///
    /// # Errors
    /// [`Error::Remote`] when the instance does not answer with an array.
    pub async fn posts(
        &self,
        host: &str,
        id: &str,
        max_id: Option<&str>,
    ) -> Result<Page<Post>, Error> {
        let suffix = max_id.map_or_else(String::new, |id| format!("&max_id={id}"));
        let value = self
            .get(
                host,
                &format!("/api/v1/accounts/{id}/statuses?limit=20{suffix}"),
            )
            .await?;
        let items = value
            .as_array()
            .ok_or_else(|| Error::Remote("statuses was not an array".to_owned()))?;
        let posts: Vec<Post> = items.iter().map(mastodon::post).collect();
        let next = posts.last().map(|post| post.id.clone());
        Ok(Page { items: posts, next })
    }

    /// Read one page of followers or following.
    ///
    /// # Errors
    /// [`Error::Remote`] when the source does not expose a collection.
    pub async fn graph(
        &self,
        host: &str,
        id: &str,
        following: bool,
    ) -> Result<Page<Account>, Error> {
        let kind = if following { "following" } else { "followers" };
        let value = self
            .get(host, &format!("/api/v1/accounts/{id}/{kind}?limit=40"))
            .await?;
        let items = value
            .as_array()
            .ok_or_else(|| Error::Remote(format!("{kind} was not an array")))?;
        Ok(Page {
            items: items.iter().map(|value| mastodon::account(host, value)).collect(),
            next: None,
        })
    }

    /// Read a thread from a Mastodon origin.
    ///
    /// # Errors
    /// [`Error::Remote`] when the context cannot be read.
    pub async fn thread(&self, host: &str, id: &str) -> Result<Vec<Post>, Error> {
        let value = self
            .get(host, &format!("/api/v1/statuses/{id}/context"))
            .await?;
        let mut posts = Vec::new();
        for key in ["ancestors", "descendants"] {
            if let Some(items) = value.get(key).and_then(serde_json::Value::as_array) {
                posts.extend(items.iter().map(mastodon::post));
            }
        }
        Ok(posts)
    }

    /// Read replies from an ActivityPub note.
    ///
    /// # Errors
    /// [`Error::Remote`] when the note or its replies cannot be read.
    pub async fn replies_acct(&self, url: &str) -> Result<Page<Post>, Error> {
        let note = self.ap.note(url).await.map_err(remote)?;
        let collection = self.ap.replies(&note).await.map_err(remote)?;
        let mut posts = Vec::new();
        for item in collection.items {
            if let Ok(note) = nodus_activitypub_sdk::Note::from_json(&item) {
                posts.push(ap_post(note));
            }
        }
        Ok(Page {
            items: posts,
            next: collection.next,
        })
    }

    async fn get(&self, host: &str, path: &str) -> Result<serde_json::Value, Error> {
        let url = format!("https://{host}{path}");
        let response = self
            .http
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|error| Error::Remote(error.to_string()))?;
        if !response.status().is_success() {
            return Err(Error::Remote(format!(
                "{url} returned {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| Error::Remote(error.to_string()))
    }
}

fn remote(error: nodus_activitypub_sdk::Error) -> Error {
    Error::Remote(error.to_string())
}

fn ap_post(note: nodus_activitypub_sdk::Note) -> Post {
    Post {
        id: note.id.clone(),
        url: note.url,
        content: note.content,
        published: note.published,
        sensitive: note.sensitive,
        spoiler_text: note.summary,
        media: note
            .attachments
            .into_iter()
            .map(|attachment| Media {
                kind: attachment.kind.to_lowercase(),
                url: attachment.url,
                preview_url: attachment.preview_url,
                description: attachment.description,
                width: attachment.width,
                height: attachment.height,
            })
            .collect(),
        emojis: note
            .emojis
            .into_iter()
            .map(|emoji| Emoji {
                shortcode: emoji.shortcode,
                url: emoji.url,
            })
            .collect(),
        replies_count: note.replies_count,
    }
}
