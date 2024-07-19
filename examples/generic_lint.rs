use std::fmt::{Debug, Display};

use warnings::*;

/// Create a warning that will only run if the lint is enabled.
#[warning]
fn lint<D: Display, E: Debug>(args: D, e: E) {
    println!("Lint called with args: {e:?}, {args}");
}

#[tokio::main]
async fn main() {
    // Anything under the allow closure will not trigger the warning
    lint::allow(|| lint(0, 0));
    // Any future executed in the future with the warning allowed will not trigger the warning
    async {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        lint(1, 1);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    .allow::<lint>()
    .await;

    // If the warning is not allowed, it will be called
    lint(2, 2);
}
