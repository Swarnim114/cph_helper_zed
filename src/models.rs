use serde::{Deserialize, Serialize};

// this is the shape of the JSON that the Competitive Companion browser extension sends us
// the field names use camelCase in the JSON (e.g. "timeLimit") but we use snake_case in Rust
// the #[serde(rename_all = "camelCase")] line handles that conversion automatically
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub name: String,        // e.g. "Two Sets"
    pub group: String,       // e.g. "Codeforces Round 123"
    pub url: String,         // link to the problem page
    pub tests: Vec<TestCase>,
    pub time_limit: u64,     // in milliseconds
    pub memory_limit: u64,   // in megabytes
}

// one sample test case - an input string and the expected output string
#[derive(Debug, Deserialize, Serialize)]
pub struct TestCase {
    pub input: String,
    pub output: String,
}
