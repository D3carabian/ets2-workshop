import type { Truck } from "./types";

// Stable buckets avoid a comparison sort and preserve save order within groups.
export function orderFleet(trucks: readonly Truck[]): Truck[] {
  const current: Truck[] = [];
  const other: Truck[] = [];
  const employees: Truck[] = [];
  for (const truck of trucks) {
    (truck.current
      ? current
      : truck.driver?.kind === "employee"
        ? employees
        : other
    ).push(truck);
  }
  return current.concat(other, employees);
}

export function newSaveName(date = new Date()): string {
  const pad = (value: number, width = 2) => String(value).padStart(width, "0");
  return `Workshop_${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}_${pad(date.getHours())}-${pad(date.getMinutes())}-${pad(date.getSeconds())}-${pad(date.getMilliseconds(), 3)}`;
}
