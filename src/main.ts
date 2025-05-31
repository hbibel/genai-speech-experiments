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
        input_audio_noise_reduction: { type: "far_field" },
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

  let speechStart: number | undefined = undefined;
  let transcriptionStart: number | undefined = undefined;

  ws.on("message", function incoming(message) {
    const event = JSON.parse(message.toString());

    switch (event["type"]) {
      case "transcription_session.updated":
        state = "waiting_for_recording";
        break;
      case "conversation.item.input_audio_transcription.completed":
        state = "conversation_done";

        if (transcriptionStart === undefined) {
          console.warn("Missing input_audio_buffer.committed event");
        } else {
          const transcriptionDuration =
            (Date.now() - transcriptionStart) / 1000;
          console.log(
            `Transcription took ${transcriptionDuration.toFixed(2)} seconds`,
          );
        }

        console.log(event);
        break;
      case "conversation.item.input_audio_transcription.delta":
        state = "waiting_for_recording";
        console.log("delta:", event["delta"]);
        break;
      case "input_audio_buffer.speech_stopped":
        if (speechStart === undefined) {
          console.warn("Missing speech_started event");
          console.log("User has stopped speaking");
        } else {
          const speechDuration = (Date.now() - speechStart) / 1000;
          console.log(
            `User has stopped speaking after ${speechDuration.toFixed(2)} seconds`,
          );
        }
        break;
      case "input_audio_buffer.speech_started":
        speechStart = Date.now();
        break;
      case "input_audio_buffer.committed":
        transcriptionStart = Date.now();
        break;
      case "transcription_session.created":
      case "conversation.item.created":
        break;
      default:
        console.warn("unhandled event", event);
    }
  });

  ws.on("close", () => console.log("Websocket closed"));

  const audioStream = startRecording();

  while (state !== "conversation_done") {
    if (state === "waiting_for_recording") {
      const data: Buffer | null = audioStream.read(4096);
      if (data !== null) {
        ws.send(
          JSON.stringify({
            type: "input_audio_buffer.append",
            audio: data.toString("base64"),
          }),
        );
      } else {
        // data === null because there are less than 4096 bytes in the audio buffer
      }
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  audioStream.destroy?.();

  ws.removeAllListeners();
  ws.close();
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
