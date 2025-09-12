use jarvis_code::actions::IntentClassifier;
use jarvis_code::ai_providers::openai::Gpt4_1Nano;
use jarvis_code::app_composite;
use jarvis_code::config;
use jarvis_code::session::Author;
use jarvis_code::session::ConversationContext;
use jarvis_code::session::TextMessage;
use jarvis_code::speech::input::Transcription;

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
        app_composite
            .logger
            .debug(format!("User said: {user_command:?}"));

        let user_text = match user_command {
            Transcription::Empty => String::default(),
            Transcription::Some { text } => text,
        };
        let conversation_context = ConversationContext {
            log: vec![TextMessage {
                author: Author::User,
                parts: vec![user_text],
            }],
        };

        let model = Gpt4_1Nano::new(config.openai_key);
        let classifier = IntentClassifier::new(model);

        classifier
            .classify_intent(&conversation_context.into())
            .await?;

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
