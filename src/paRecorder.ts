import { spawn } from "node:child_process";
import { Readable } from "node:stream";

export function startRecording(): Readable {
  const proc = spawn("parec", [
    "--format=s16le",
    "--rate=24000",
    "--channels=1",
  ]);
  return proc.stdout;
}
