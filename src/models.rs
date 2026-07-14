use serde::{Deserialize, Serialize};

// Matches the JSON payload sent by the Competitive Companion browser extension
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub name: String,
    pub group: String,
    pub url: String,
    pub tests: Vec<TestCase>,
    pub time_limit: u64,
    pub memory_limit: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TestCase {
    pub input: String,
    pub output: String,
}
