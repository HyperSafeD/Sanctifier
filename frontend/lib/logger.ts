/**
 * Structured Logger for Sanctifier Frontend API Routes
 * Emits JSON logs formatted according to the unified telemetry schema:
 * {
 *   "timestamp": "2026-08-31T04:00:00.000Z",
 *   "level": "info" | "warn" | "error" | "debug",
 *   "request_id": "req-1234-5678",
 *   "component": "frontend-api",
 *   "message": "Event description",
 *   ...additionalContext
 * }
 */

export type LogLevel = "info" | "warn" | "error" | "debug";

export interface LogEventContext {
  request_id?: string;
  component?: string;
  path?: string;
  method?: string;
  status_code?: number;
  duration_ms?: number;
  [key: string]: unknown;
}

class StructuredLogger {
  private component: string;

  constructor(component = "frontend-api") {
    this.component = component;
  }

  private log(level: LogLevel, message: string, context: LogEventContext = {}) {
    const payload = {
      timestamp: new Date().toISOString(),
      level,
      request_id: context.request_id || "req-system-generated",
      component: context.component || this.component,
      message,
      ...context,
    };

    const output = JSON.stringify(payload);

    if (level === "error") {
      console.error(output);
    } else if (level === "warn") {
      console.warn(output);
    } else {
      console.log(output);
    }
  }

  info(message: string, context?: LogEventContext) {
    this.log("info", message, context);
  }

  warn(message: string, context?: LogEventContext) {
    this.log("warn", message, context);
  }

  error(message: string, context?: LogEventContext) {
    this.log("error", message, context);
  }

  debug(message: string, context?: LogEventContext) {
    if (process.env.NODE_ENV !== "production") {
      this.log("debug", message, context);
    }
  }
}

export const logger = new StructuredLogger();
