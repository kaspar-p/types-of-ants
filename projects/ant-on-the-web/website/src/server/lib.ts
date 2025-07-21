import { z } from "zod";

const environmentSchema = z.union([
  z.literal("dev"),
  z.literal("beta"),
  z.literal("prod"),
]);

function generateBaseUrl(): string {
  const environment = environmentSchema.parse(
    process.env.NEXT_PUBLIC_TYPESOFANTS_ENV
  );

  let url: string;
  switch (environment) {
    case "prod": {
      url = `https://typesofants.org`;
      break;
    }
    case "beta": {
      const port = z.string().parse(process.env.NEXT_PUBLIC_ANT_GATEWAY_PORT);
      url = `https://beta.typesofants.org:${port}`;
      break;
    }
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
  return new URL(baseUrl + path);
}

export function getFetchOptions(): { credentials?: "include" } {
  const environment = environmentSchema.parse(
    process.env.NEXT_PUBLIC_TYPESOFANTS_ENV
  );
  switch (environment) {
    case "prod":
    case "beta":
      return {};
    case "dev":
      return { credentials: "include" };
  }
}
