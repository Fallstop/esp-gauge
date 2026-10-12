mod claude;
mod codex;
use super::Feed;

#[derive(Default)]
pub struct Quotas {
    claude: claude::Claude,
    codex: codex::Codex,
}
impl Quotas {
    pub fn sample(&mut self) -> Feed {
        let mut feed = Feed::default();
        self.codex.sample(&mut feed);
        self.claude.sample(&mut feed);
        feed
    }
}
pub(super) fn local(t: chrono::DateTime<chrono::Utc>) -> String {
    t.with_timezone(&chrono::Local)
        .format("%a %H:%M")
        .to_string()
}
