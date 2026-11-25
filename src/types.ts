export * from "./types.gen";

/** State of the notification permission. Only Android asks for it, elsewhere it is `unsupported`. */
export type NotificationPermission = "granted" | "prompt" | "denied" | "unsupported";
