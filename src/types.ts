export * from "./types.gen";

export type ReceivingPairingCode = {
  code: string;
  expiresInMs: number;
};

/** State of the notification permission. Only Android asks for it, elsewhere it is `unsupported`. */
export type NotificationPermission = "granted" | "prompt" | "denied" | "unsupported";
