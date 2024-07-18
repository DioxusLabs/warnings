use warnings::*;

#[warning]
fn lint() {
    let sum = 1 + 1;
    println!("{}", sum);
    panic!();
}

#[tokio::main]
async fn main() {
    // Anything under the allow closure will not trigger the lint
    allow(&lint, *lint);
    // Any future executed in the future with the lint allowed will not trigger the lint
    async {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        lint();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    .allow(&lint)
    .await;
    lint();
}
