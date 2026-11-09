//! DiceBear's public HTTP API: a drawn avatar for any name, in a chosen style. Only its CC0
//! styles are used, so no credit is owed. Free for non-commercial use, at most 10 PNGs a second.
use std::time::Duration;

use reqwest::Client;

const BASE_URL: &str = "https://api.dicebear.com/10.x";
/// An avatar takes no longer than this, or that player goes without one.
const TIMEOUT: Duration = Duration::from_secs(15);
const USER_AGENT: &str = "chess-backend (a personal chess app)";
/// The largest PNG DiceBear draws.
const SIDE: u32 = 256;

#[derive(Clone)]
pub struct DiceBearClient {
    http: Client,
}

impl DiceBearClient {
    pub fn new() -> anyhow::Result<Self> {
        let http = Client::builder()
            .timeout(TIMEOUT)
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Self { http })
    }

    /// The PNG avatar `style` draws for `seed`, a UUID, so it needs no escaping in the address;
    /// the same seed draws the same face.
    pub async fn avatar_png(&self, style: &str, seed: uuid::Uuid) -> anyhow::Result<Vec<u8>> {
        let png = self
            .http
            .get(format!("{BASE_URL}/{style}/png?seed={seed}&size={SIDE}"))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(png.to_vec())
    }
}
