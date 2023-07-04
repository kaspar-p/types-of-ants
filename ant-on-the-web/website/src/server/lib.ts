import { z } from "zod";

function generateBaseUrl() {
  const environment = z
    .union([z.literal("dev"), z.literal("beta")])
    .parse(process.env.ENVIRONMENT ?? process.env.NEXT_PUBLIC_ENVIRONMENT);

  switch (environment) {
    case "beta":
      return "https://beta.typesofants.org";
    case "dev":
      return "http://localhost:3499";
  }
}

// TODO: Find a better solution for dev/beta/prod machines
export function getEndpoint(path: string) {
  const baseUrl = generateBaseUrl();
  if (path[0] !== "/") path = "/" + path;
  return baseUrl + path;
}
