export type BufferState = 'live' | 'stale' | 'blocked' | 'replaced' | 'killed';
export type FrameKind = 'sveltekit' | 'tui' | 'emacs' | 'other';
export type MajorMode =
  | 'table'
  | 'list'
  | 'graph'
  | 'timeline'
  | 'log'
  | 'markdown'
  | 'document'
  | 'browser'
  | 'image'
  | 'artifact'
  | 'custom';
export type PayloadContract = 'framed-json' | 'typed-wit' | 'hybrid';
export type WindowConnectionState = 'connected' | 'disconnected';

export interface FrameLocals {}

export interface ViewRequirement {
  fact_path: string;
  required: boolean;
  purpose: string;
}

export interface ViewShape {
  shape_id: string;
  title: string;
  source_ref: string;
  scope: string;
  version: number;
  active: boolean;
  major_mode: MajorMode;
  minor_modes: string[];
  maturity: string;
  payload_contract: PayloadContract;
  payload_version: number;
  vision_id?: string | null;
  project_uid?: string | null;
  replaced_by?: string | null;
  requirements: ViewRequirement[];
}

export interface ViewBuffer {
  buffer_id: string;
  name: string;
  shape_id: string;
  state: BufferState;
  created_at: string;
  stale_at?: string | null;
  blocked_at?: string | null;
  replaced_at?: string | null;
  killed_at?: string | null;
  replacement_buffer_id?: string | null;
  major_mode: MajorMode;
  minor_modes: string[];
  payload_contract: PayloadContract;
  payload_version: number;
}

export interface ViewWindow {
  window_id: string;
  frame_id: string;
  buffer_id?: string | null;
  connection_state: WindowConnectionState;
  connected_at?: string | null;
  disconnected_at?: string | null;
}

export interface ObservabilityGap {
  gap_id: string;
  shape_id?: string | null;
  missing_fact_path: string;
  missing_source_id?: string | null;
  reason: string;
  status: string;
  linked_work_item_id?: string | null;
  created_at: string;
  resolved_at?: string | null;
}

export interface PayloadFrame {
  protocol: 'patina:view-buffer';
  version: number;
  payload_contract: PayloadContract;
  shape_id: string;
  shape_version: number;
  buffer_id: string;
  payload_version: number;
}

export interface FramedJsonPayload {
  frame: PayloadFrame;
  json: unknown;
}

export interface OpenedBuffer {
  buffer: ViewBuffer;
  payload: FramedJsonPayload;
}

export interface ViewRequestAction {
  kind: 'open_matched_shape' | 'open_adapted_shape' | 'open_created_shape';
  shape_id: string;
  label: string;
}

export interface ViewRequestDetail {
  request: {
    request_id: string;
    user_id: string;
    agent_id: string;
    raw_request: string;
    requested_at: string;
    outcome: string;
  };
  available_actions: ViewRequestAction[];
  shape_match?: { shape_id?: string | null; match_kind: string; confidence: number };
  adapted_shape?: ViewShape;
  created_shape?: ViewShape;
}

export interface ConnectWindowRequest {
  frame_id: string;
  frame_kind: FrameKind;
  window_id: string;
  buffer_id: string;
}

export interface DisconnectWindowRequest {
  window_id: string;
}

export interface OpenBufferRequest {
  shape_id: string;
}

export interface OpenRequestShapeRequest {
  request_id: string;
  shape_id?: string;
}

export interface OpenRequestShapeOutcome {
  request_id: string;
  shape_id: string;
  open_outcome: unknown;
}

export interface ApiErrorPayload {
  error?: string;
  message?: string;
  gap?: ObservabilityGap;
}
