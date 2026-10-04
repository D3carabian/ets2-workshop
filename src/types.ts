export type Settings = {
  documents: string;
  game: string;
  extractor: string;
  onboarding_version: number;
};
export type Definition = {
  path: string;
  kind: string;
  unit: string;
  name: string;
  raw_name?: string;
  names?: Record<string, string>;
  category_names?: Record<string, string>;
  name_alias?: string | null;
  category: string;
  model: string;
  source: string;
  metrics: Record<string, string>;
  suitable: string[];
  conflicts: string[];
  requires: string[];
};
export type Accessory = {
  id: string;
  index: number;
  kind: string;
  path: string;
  category: string;
  name: string;
  model: string;
  refund: string;
  raw: string;
  definition: Definition | null;
};
export type Truck = {
  id: string;
  plate: string;
  model: string;
  current: boolean;
  location: string;
  driver: {
    kind: "player" | "employee" | "unassigned" | "unknown";
    id: string | null;
    name: string | null;
  };
  accessories: Accessory[];
};
export type Opened = {
  path: string;
  hash: string;
  trucks: Truck[];
  warnings: string[];
};
export type SaveEntry = {
  is_autosave?: boolean;
  path: string;
  name: string;
  profile: string;
  modified: number;
  error: string | null;
};
export type Operation = {
  truck_id: string;
  action: "replace" | "add";
  accessory_id: string | null;
  candidate_path: string;
  donor_accessory: string | null;
};
export type Change = {
  truck_id: string;
  model: string;
  plate: string;
  category: string;
  action: string;
  before: string;
  after: string;
};
export type Preview = {
  changes: Change[];
  warnings: string[];
  trucks: Truck[];
};
export type Receipt = {
  id: string;
  source: string;
  output: string;
  backup: string;
  state: "preparing" | "prepared" | "completed";
  warning: string | null;
  info_hash: string | null;
  before_hash: string;
  after_hash: string;
  changes: Change[];
};
export type Verification = {
  category: string;
  expected: string;
  status: string;
  matches: string[];
};
