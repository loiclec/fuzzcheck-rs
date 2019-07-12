#[derive(Debug)]
pub enum ArtifactKind {
    Input,
    Timeout,
    Crash,
    TestFailure,
}
