export * from "./types.gen";

// UI helpers

export interface PersonalDate {
  id?: string;
  month: number; // 1-12
  day: number; // 1-31
  label: string;
  year: number;
}
