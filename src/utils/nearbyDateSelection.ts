import type { PersonalDate } from "../types";

/**
 * Pure utilities for picking a "nearby" personal date to show when there's
 * no exact match for today. Extracted so the weighting math can be
 * unit-tested without mocking React, Date, or Math.random.
 */

/** Multiplier applied to the date that was shown last time, to avoid repeats. */
export const RECENTLY_SHOWN_PENALTY = 0.3;

export function isLeapYear(year: number): boolean {
  return (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
}

/**
 * Day-of-year (1-366) for a month/day pair in a given year.
 * Feb 29 in a non-leap year is folded into Feb 28's slot — this is the
 * one place that rule lives; don't reimplement it elsewhere.
 */
export function dayOfYear(year: number, month: number, day: number): number {
  const leap = isLeapYear(year);
  const normalizedDay = !leap && month === 2 && day === 29 ? 28 : day;
  const daysInMonth = [0, 31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

  let doy = normalizedDay;
  for (let m = 1; m < month; m++) doy += daysInMonth[m];
  return doy;
}

/** Shortest distance, in days, between two month/day pairs on a wrap-around (circular) year. */
export function circularDistance(
  year: number,
  m1: number,
  d1: number,
  m2: number,
  d2: number,
): number {
  const a = dayOfYear(year, m1, d1);
  const b = dayOfYear(year, m2, d2);
  const diff = Math.abs(a - b);
  const daysInYear = isLeapYear(year) ? 366 : 365;
  return Math.min(diff, daysInYear - diff);
}

/** Reciprocal decay: distance 0 -> weight 1, distance N -> weight 1/(N+1). */
function distanceWeight(distanceInDays: number): number {
  return 1 / (distanceInDays + 1);
}

interface WeightedDate {
  date: PersonalDate;
  weight: number;
}

/**
 * Attach a selection weight to each date, based on its distance from `today`
 * and a penalty if it was the last one shown.
 * Pure — safe to unit-test directly with fixed inputs.
 */
export function weighDates(
  dates: PersonalDate[],
  today: Date,
  lastShownId: string | null,
): WeightedDate[] {
  const year = today.getFullYear();
  const month = today.getMonth() + 1;
  const day = today.getDate();

  return dates.map((date) => {
    const distance = circularDistance(year, month, day, date.month, date.day);
    const base = distanceWeight(distance);
    const wasLastShown = lastShownId != null && date.id === lastShownId;
    return { date, weight: wasLastShown ? base * RECENTLY_SHOWN_PENALTY : base };
  });
}

/**
 * Generic weighted random pick. `rng` defaults to Math.random but accepts
 * an injected function so callers can test with fixed "rolls".
 */
export function pickWeighted<T>(
  items: { item: T; weight: number }[],
  rng: () => number = Math.random,
): T | null {
  if (items.length === 0) return null;

  const total = items.reduce((sum, i) => sum + i.weight, 0);
  let roll = rng() * total;

  for (const { item, weight } of items) {
    roll -= weight;
    if (roll <= 0) return item;
  }
  return items[items.length - 1].item; // floating-point rounding fallback
}

/** Pick a personal date to display when there's no exact match for today. */
export function selectNearbyDate(
  dates: PersonalDate[],
  today: Date,
  lastShownId: string | null,
  rng: () => number = Math.random,
): PersonalDate | null {
  if (dates.length === 0) return null;
  const weighted = weighDates(dates, today, lastShownId);
  return pickWeighted(
    weighted.map(({ date, weight }) => ({ item: date, weight })),
    rng,
  );
}
