import { z } from "zod";

const environmentSchema = z.union([
  z.literal("dev"),
  z.literal("beta"),
  z.literal("prod"),
  z.undefined(),
]);

function generateBaseUrl(): string {
  const environment = environmentSchema.parse(
    process.env.NEXT_PUBLIC_TYPESOFANTS_ENV,
  );

  let url: string;
  switch (environment) {
    case "prod": {
      // Let the browser route itself: https://github.com/whatwg/url/issues/531
      url = document.baseURI;
      break;
    }
    case "beta": {
      const port = z.string().parse(process.env.NEXT_PUBLIC_ANT_GATEWAY_PORT);
      url = `https://beta.typesofants.org:${port}`;
      break;
    }
    case undefined:
    case "dev": {
      const port = z
        .string()
        .parse(process.env.NEXT_PUBLIC_ANT_ON_THE_WEB_PORT);
      url = `http://localhost:${port}`;
      break;
    }
    default: {
      throw new Error("Unsupported environment: " + environment);
    }
  }

  return url;
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
    case "prod":
    case "beta": {
      return {};
    }

    case undefined:
    case "dev": {
      return {
        credentials: "include",
        headers: headers,
      };
    }
  }
}
