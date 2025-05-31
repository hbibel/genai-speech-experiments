import { TranscriptionSessionUpdate } from "openai/resources/beta/realtime/realtime";
import WebSocket from "ws";
import { startRecording } from "./paRecorder";

type State =
  | "initializing"
  | "waiting_for_recording"
  | "waiting_for_openai"
  | "conversation_done";

async function main() {
  const openaiApiKey = process.env.OPENAI_SHENANIGANS;
  if (openaiApiKey === undefined) {
    throw Error("env variable OPENAI_SHENANIGANS must be set");
  }

  const url = "wss://api.openai.com/v1/realtime?intent=transcription";
  const ws = new WebSocket(url, {
    headers: {
      Authorization: `Bearer ${openaiApiKey}`,
      "OpenAI-Beta": "realtime=v1",
    },
  });

  let state: State = "initializing";

  ws.on("open", function open() {
    console.log("Connected to server.");

    const sessionConfig: TranscriptionSessionUpdate = {
      session: {
        input_audio_noise_reduction: { type: "near_field" },
        input_audio_transcription: {
          language: "en",
          model: "gpt-4o-mini-transcribe",
          prompt: "expect words related to technology",
        },
        turn_detection: {
          type: "semantic_vad",
        },
      },
      type: "transcription_session.update",
    };
    ws.send(JSON.stringify(sessionConfig));
  });

  ws.on("message", function incoming(message) {
    const event = JSON.parse(message.toString());
    console.log(event);

    switch (event["type"]) {
      case "transcription_session.updated":
        state = "waiting_for_recording";
        break;
      case "conversation.item.input_audio_transcription.completed":
        state = "conversation_done";
        break;
      case "conversation.item.input_audio_transcription.delta":
        state = "waiting_for_recording";
        console.log("delta:", event["delta"]);
        break;
      case "input_audio_buffer.speech_stopped":
        console.log("User has stopped speaking");
        break;
      case "transcription_session.created":
      case "input_audio_buffer.speech_started":
      case "input_audio_buffer.committed":
        // ignore
        break;
      default:
        console.warn("unhandled event");
    }
  });

  ws.on("close", () => console.log("Websocket closed"));

  const audioStream = startRecording();

  while (state !== "conversation_done") {
    if (state === "waiting_for_recording") {
      const data: Buffer | null = audioStream.read();
      if (data !== null) {
        ws.send(
          JSON.stringify({
            type: "input_audio_buffer.append",
            audio: data.toString("base64"),
          }),
        );
      } else {
        console.log("not enough data");
      }
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  console.log("shutting down");
  ws.close();
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
