import { NextRequest } from "next/server";
import { spawn } from "child_process";
import path from "path";

export async function GET(request: NextRequest) {
    const searchParams = request.nextUrl.searchParams;
    const projectPath = searchParams.get("path") || ".";

    // In a real environment, we'd want to validate the path to prevent security issues.
    // For this exercise, we'll assume the path is safe or controlled.

    const encoder = new TextEncoder();
    const stream = new ReadableStream({
        start(controller) {
            // Find the sanctifier binary. Assuming it's in the workspace root or path.
            // For development, we might need to run 'cargo run --bin sanctifier' 
            // but let's assume 'sanctifier' is in the PATH or we can find it.

            const cliProcess = spawn("cargo", ["run", "--bin", "sanctifier", "--", "analyze", "--path", projectPath], {
                cwd: "/home/rampop/Sanctifier", // Run from project root
                env: { ...process.env, FORCE_COLOR: "0" }, // Disable colors for simpler parsing initially
            });

            const sendLog = (data: string) => {
                const lines = data.split("\n");
                for (const line of lines) {
                    if (line.trim()) {
                        controller.enqueue(encoder.encode(`data: ${JSON.stringify(line)}\n\n`));
                    }
                }
            };

            cliProcess.stdout.on("data", (data) => {
                sendLog(data.toString());
            });

            cliProcess.stderr.on("data", (data) => {
                sendLog(`[DEBUG] ${data.toString()}`);
            });

            cliProcess.on("close", (code) => {
                controller.enqueue(encoder.encode(`data: ${JSON.stringify(`--- Analysis complete with exit code ${code} ---`)}\n\n`));
                controller.close();
            });

            cliProcess.on("error", (err) => {
                controller.enqueue(encoder.encode(`data: ${JSON.stringify(`Error spawning process: ${err.message}`)}\n\n`));
                controller.close();
            });
        },
    });

    return new Response(stream, {
        headers: {
            "Content-Type": "text/event-stream",
            "Cache-Control": "no-cache",
            Connection: "keep-alive",
        },
    });
}
