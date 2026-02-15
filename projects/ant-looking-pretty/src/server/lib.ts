import { z } from "zod";

const environmentSchema = z.string().optional();

function generateBaseUrl(): string {
  const environment = environmentSchema.parse(
    process.env.NEXT_PUBLIC_TYPESOFANTS_ENV,
  );

  switch (environment) {
    case "dev": {
      return `http://localhost:3231`;
    }

    default: {
      // Let the browser route itself: https://github.com/whatwg/url/issues/531
      return document.baseURI;
    }
  }
}

// TODO: Find a better solution for dev/beta/prod machines
export function getEndpoint(path: string): URL {
  const baseUrl: string = generateBaseUrl();
  if (path[0] !== "/") path = "/" + path;
  return new URL(path, baseUrl);
}

export async function getFetchOptions(): Promise<RequestInit> {
  const environment = environmentSchema.parse(
    process.env.NEXT_PUBLIC_TYPESOFANTS_ENV,
  );

  const headers = {
    Cookie:
      typeof window === "undefined"
        ? (await (await import("next/headers")).cookies()).toString()
        : "",
  };

  switch (environment) {
    case "dev": {
      return {
        credentials: "include",
        headers: headers,
      };
    }

    default: {
      return {};
    }
  }
}
