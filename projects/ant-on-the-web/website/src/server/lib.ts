import { z } from "zod";

function generateBaseUrl() {
  const environment = z
    .union([z.literal("dev"), z.literal("beta")])
    .parse(process.env.ENVIRONMENT ?? process.env.NEXT_PUBLIC_ENVIRONMENT);

  switch (environment) {
    case "beta":
      return "https://beta.typesofants.org";
    case "dev":
      if (!process.env.NEXT_PUBLIC_ANT_ON_THE_WEB_PORT) {
        console.log(process.env);
        throw new Error(
          "Require NEXT_PUBLIC_ANT_ON_THE_WEB_PORT environment variable."
        );
      }
      return `http://localhost:${process.env.NEXT_PUBLIC_ANT_ON_THE_WEB_PORT}`;
  }
}

// TODO: Find a better solution for dev/beta/prod machines
export function getEndpoint(path: string): URL {
  const baseUrl = generateBaseUrl();
  if (path[0] !== "/") path = "/" + path;
  return new URL(baseUrl + path);
}

export function getFetchOptions(): { credentials?: "include" } {
  const environment = z
    .union([z.literal("dev"), z.literal("beta"), z.literal("prod")])
    .parse(process.env.ENVIRONMENT ?? process.env.NEXT_PUBLIC_ENVIRONMENT);
  switch (environment) {
    case "prod":
    case "beta":
      return {};
    case "dev":
      return { credentials: "include" };
  }
}
