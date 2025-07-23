use jarvis_code::app_composite;
use jarvis_code::config;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    // Initialize rustls crypto provider, for secure connections
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    let config = config::from_env()?;

    let mut app_composite = app_composite::AppComposite::new(&config)?;

    loop {
        let user_command = app_composite.speech_listener.listen_to_input().await?;

        println!("User said: {user_command:?}");

        todo!("for now let's run the loop only once")
        // user_command = get_user_input()
        // action = generate_action()
        // render_action()?;
        // user_feedback = get_action_feedback();
        // while !user_feedback is affirmative {
        //      action = generate_alternative_action()
        // }
        // execute(action)
        // save_lessons_learned(action)
    }
}
