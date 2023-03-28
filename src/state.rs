use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use oauth2::{
    AuthUrl, ClientId, ClientSecret, PkceCodeVerifier, RedirectUrl, RevocationUrl, TokenUrl,
};
use tower_cookies::Key;

use crate::Config;

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct AppState {
    pub tokens: Tokens,
    pub oauth: oauth2::basic::BasicClient,
    pub key: Arc<Key>,
    pub tera: Arc<tera::Tera>,
    pub client: ClassroomHttpClient,
}

pub type ClassroomHttpClient =
    classroom::hyper::client::Client<ClassroomHyperClient, classroom::hyper::Body>;
pub type ClassroomHyperClient =
    classroom::hyper_rustls::HttpsConnector<classroom::hyper::client::HttpConnector>;
pub type Tokens = Arc<RwLock<HashMap<String, PkceCodeVerifier>>>;

impl AppState {
    /// Create a new [`AppState`].
    /// # Panics
    /// This function panics on invalid templates.
    pub fn new(config: Config) -> Self {
        let mut tera = tera::Tera::new("templates/*").expect("Failed to create templates");
        tera.autoescape_on(vec!["xml", "htm", "html", "jinja", "jinja2"]);
        let tera = Arc::new(tera);
        crate::error::ERROR_TERA.try_insert(tera.clone()).ok();
        let oauth = oauth2::basic::BasicClient::new(
            ClientId::new(config.client_id),
            Some(ClientSecret::new(config.client_secret)),
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap(),
            Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap()),
        )
        .set_revocation_uri(
            RevocationUrl::new("https://oauth2.googleapis.com/revoke".to_string()).unwrap(),
        )
        // Set the URL the user will be redirected to after the authorization process.
        .set_redirect_uri(
            RedirectUrl::new(format!(
                "{}/oauth/callback",
                config.root_url.trim_end_matches('/')
            ))
            .unwrap(),
        );
        let key_bytes = hex::decode(config.key).expect("Invalid hex");
        let key = Arc::new(Key::from(&key_bytes));
        let tokens: Tokens = Arc::new(RwLock::new(HashMap::new()));
        let client = classroom::hyper::Client::builder().build(
            classroom::hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_only()
                .enable_http1()
                .build(),
        );
        Self {
            tokens,
            oauth,
            key,
            tera,
            client,
        }
    }
}
