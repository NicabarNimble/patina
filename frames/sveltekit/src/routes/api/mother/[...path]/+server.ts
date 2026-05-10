import { env } from '$env/dynamic/private';
import { json, type RequestHandler } from '@sveltejs/kit';

function configuredBaseUrl(): string {
  const raw = env.MOTHER_API_BASE_URL ?? env.PATINA_MOTHER ?? 'http://127.0.0.1:50051';
  if (raw.startsWith('http://') || raw.startsWith('https://')) {
    return raw;
  }
  return `http://${raw}`;
}

function bearerToken(): string | undefined {
  return env.MOTHER_API_TOKEN ?? env.PATINA_MOTHER_TOKEN;
}

function motherUrl(path: string, search: string): URL {
  const url = new URL(`/api/${path}${search}`, configuredBaseUrl());
  return url;
}

async function forward({ request, params, url, fetch }: Parameters<RequestHandler>[0]): Promise<Response> {
  const path = params.path ?? '';
  if (!path || path.includes('..')) {
    return json({ error: 'invalid Mother API path' }, { status: 400 });
  }

  const headers = new Headers();
  headers.set('accept', 'application/json');
  const contentType = request.headers.get('content-type');
  if (contentType) {
    headers.set('content-type', contentType);
  }
  const token = bearerToken();
  if (token) {
    headers.set('authorization', `Bearer ${token}`);
  }

  const init: RequestInit = {
    method: request.method,
    headers
  };
  if (request.method !== 'GET' && request.method !== 'HEAD') {
    init.body = await request.text();
  }

  try {
    const upstream = await fetch(motherUrl(path, url.search), init);
    const body = await upstream.text();
    return new Response(body, {
      status: upstream.status,
      headers: {
        'content-type': upstream.headers.get('content-type') ?? 'application/json'
      }
    });
  } catch (error) {
    return json(
      {
        error: 'mother_unreachable',
        message: error instanceof Error ? error.message : String(error)
      },
      { status: 502 }
    );
  }
}

export const GET: RequestHandler = forward;
export const POST: RequestHandler = forward;
