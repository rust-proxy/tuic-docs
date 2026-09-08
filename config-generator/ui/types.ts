// Display-only contract of session/view.rs. Conditions and configuration values stay in Rust.
export interface FieldView {
  key: string;
  path: string;
  label: string;
  hint: string;
  placeholder: string;
  kind: 'text' | 'password' | 'number' | 'email' | 'toggle' | 'select';
  options: [string, string][];
  value: string;
  visible: boolean;
  error: string;
  generated: boolean;
}

export interface RowView {
  id: string;
  number: number;
  visible: boolean;
  fields: FieldView[];
}

export interface CollectionView {
  name: string;
  label: string;
  hint: string;
  add_label: string;
  generate_label: string;
  generated: boolean;
  visible: boolean;
  editable: boolean;
  removable: boolean;
  selector: FieldView | null;
  rows: RowView[];
}

export interface NoticeView { text: string; visible: boolean }
export interface SectionView {
  name: string;
  label: string;
  detail: string;
  collapsed: boolean;
  visible: boolean;
  fields: FieldView[];
  collections: CollectionView[];
  notices: NoticeView[];
}

export interface Snapshot {
  ui: { title: string; brand: string; mark: string; eyebrow: string; description: string; reference: string; export_hint: string };
  sections: SectionView[];
  mode: FieldView | null;
  format: FieldView | null;
  notices: NoticeView[];
  errors: Record<string, string>;
  outputs: { name: string; label: string; visible: boolean }[];
  selected: string;
  filename: string;
  command: string;
  preview: string;
  valid: boolean;
}

export type Action =
  | { type: 'set'; field: string; value: string }
  | { type: 'set-row'; collection: string; id: string; field: string; value: string }
  | { type: 'add'; collection: string }
  | { type: 'remove'; collection: string; id: string }
  | { type: 'generate'; field: string }
  | { type: 'generate-row'; collection: string; id: string };

export type Dispatch = (action: Action) => void;
export interface ExportFile { filename: string; text: string }
