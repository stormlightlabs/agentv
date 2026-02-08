use std::io::{self, Write};

const SUPPORT_MESSAGE: &str = r#"
Stormlight Labs is just me, Owais (https://desertthunder.dev). Software
like Agent V is free to use, funded by you. If Agent V saves you time or
helps your workflow, consider supporting its continued development and
my other open source work:

Support options:
  GitHub Sponsors: https://github.com/sponsors/desertthunder
  Ko-fi:           https://ko-fi.com/desertthunder

"#;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", SUPPORT_MESSAGE);

    println!("\nOpen support page in browser? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    if input == "y" || input == "yes" {
        open_support_page()?;
        println!("Opening GitHub Sponsors page...");
    } else {
        println!("You can support Agent V anytime by running: agent-v support");
    }

    Ok(())
}

fn open_support_page() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://github.com/sponsors/desertthunder";

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd").args(["/C", "start", url]).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }

    Ok(())
}
