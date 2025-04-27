import { z } from "zod";

function generateBaseUrl() {
  const environment = z
    .union([z.literal("dev"), z.literal("beta")])
    .parse(process.env.ENVIRONMENT ?? process.env.NEXT_PUBLIC_ENVIRONMENT);

  switch (environment) {
    case "beta":
      return "https://beta.typesofants.org";
    case "dev":
      if (!process.env.ANT_ON_THE_WEB_PORT) {
        throw new Error("Require ANT_ON_THE_WEB_PORT environment variable.");
      }
      return `http://localhost:${process.env.ANT_ON_THE_WEB_PORT}`;
  }
}

// TODO: Find a better solution for dev/beta/prod machines
export function getEndpoint(path: string): URL {
  const baseUrl = generateBaseUrl();
  if (path[0] !== "/") path = "/" + path;
  return new URL(baseUrl + path);
}
