import type {
  ConnectWindowRequest,
  DisconnectWindowRequest,
  ObservabilityGap,
  OpenBufferRequest,
  OpenedBuffer,
  OpenRequestShapeOutcome,
  OpenRequestShapeRequest,
  ViewBuffer,
  ViewRequestDetail,
  ViewShape,
  ViewWindow
} from './types';

class MotherApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly payload: unknown
  ) {
    super(message);
    this.name = 'MotherApiError';
  }
}

async function parseResponse<T>(response: Response): Promise<T> {
  const contentType = response.headers.get('content-type') ?? '';
  const payload = contentType.includes('application/json') ? await response.json() : await response.text();
  if (!response.ok) {
    const message =
      typeof payload === 'object' && payload !== null && 'error' in payload
        ? String((payload as { error?: unknown }).error)
        : `Mother API request failed with ${response.status}`;
    throw new MotherApiError(message, response.status, payload);
  }
  return payload as T;
}

async function get<T>(path: string): Promise<T> {
  const response = await fetch(`/api/mother/${path}`, { headers: { accept: 'application/json' } });
  return parseResponse<T>(response);
}

async function post<T>(path: string, body: unknown): Promise<T> {
  const response = await fetch(`/api/mother/${path}`, {
    method: 'POST',
    headers: { accept: 'application/json', 'content-type': 'application/json' },
    body: JSON.stringify(body)
  });
  return parseResponse<T>(response);
}

export async function listBuffers(): Promise<ViewBuffer[]> {
  return (await get<{ buffers: ViewBuffer[] }>('view-buffers')).buffers;
}

export async function getBufferPayload(bufferId: string): Promise<OpenedBuffer> {
  return (await get<{ opened: OpenedBuffer }>(`view-buffers/${encodeURIComponent(bufferId)}/payload`)).opened;
}

export async function listShapes(): Promise<ViewShape[]> {
  return (await get<{ shapes: ViewShape[] }>('view-shapes')).shapes;
}

export async function listWindows(): Promise<ViewWindow[]> {
  return (await get<{ windows: ViewWindow[] }>('view-buffers/windows')).windows;
}

export async function listGaps(): Promise<ObservabilityGap[]> {
  return (await get<{ gaps: ObservabilityGap[] }>('view-buffers/gaps')).gaps;
}

export async function listRequestDetails(): Promise<ViewRequestDetail[]> {
  return (await get<{ details: ViewRequestDetail[] }>('view-requests/details')).details;
}

export async function openBuffer(request: OpenBufferRequest): Promise<OpenedBuffer> {
  return post<OpenedBuffer>('view-buffers/open', request);
}

export async function openRequestShape(
  request: OpenRequestShapeRequest
): Promise<OpenRequestShapeOutcome> {
  return post<OpenRequestShapeOutcome>('view-requests/open-shape', request);
}

export async function connectWindow(request: ConnectWindowRequest): Promise<ViewWindow> {
  return post<ViewWindow>('view-buffers/connect', request);
}

export async function disconnectWindow(request: DisconnectWindowRequest): Promise<ViewWindow> {
  return post<ViewWindow>('view-buffers/disconnect', request);
}

export { MotherApiError };
