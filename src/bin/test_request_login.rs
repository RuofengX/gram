use gram::{scraper::Scraper, stdin_read_line};

include!("../../.config.rs");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_config = &TEST_CONFIG.into();
    let client = Scraper::new(api_config).await.unwrap();

    let phone = stdin_read_line("请输入手机号");
    let code = stdin_read_line("请输入验证码");

    let u = client
        .login(phone.await.unwrap().as_str(), code)
        .await
        .unwrap();

    let u = serde_json::to_string_pretty(&u).unwrap();
    println!("{}", u);

    let freeze = client.freeze();
    freeze.dump("./test.session")?;

    Ok(())

    // todo!()
}
