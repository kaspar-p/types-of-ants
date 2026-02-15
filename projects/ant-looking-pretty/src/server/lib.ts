import { isServer } from "@tanstack/react-query";
import { PHASE_PRODUCTION_BUILD } from "next/dist/shared/lib/constants";
import { z } from "zod";

const environmentSchema = z.string().optional();

const isBuildTime = process.env.NEXT_PHASE === PHASE_PRODUCTION_BUILD;
const isClientSide = typeof document !== "undefined";

function generateBaseUrl(): string {
  const environment = environmentSchema.parse(
    process.env.NEXT_PUBLIC_TYPESOFANTS_ENV,
  );

  switch (environment) {
    case "dev": {
      return `http://localhost:3231`;
    }

    default: {
      if (isClientSide) {
        // If this code is running client-side, keep talking to the same endpoint that the user
        // navigated to originally, e.g. "https://beta.typesofants.org:2053"
        return document.baseURI;
      } else if (isBuildTime) {
        // If this code is running at build-time, return a fake URL to prevent needing to lookup env-vars.
        return "http://fake.local:1";
      } else {
        // If the code is running server-side, use present env-vars host/port information to route
        // to the right backend
        const host = z.string().parse(process.env.ANT_ON_THE_WEB_HOST);
        const port = z.string().parse(process.env.ANT_ON_THE_WEB_PORT);
        return `http://${host}:${port}`;
      }
    }
  }
}

export function getEndpoint(path: string): URL {
  if (path[0] !== "/") path = "/" + path;
  return new URL(path, generateBaseUrl());
}

export async function getFetchOptions(): Promise<RequestInit> {
  const environment = environmentSchema.parse(
    process.env.NEXT_PUBLIC_TYPESOFANTS_ENV,
  );

  const headers = {
    Cookie: isServer
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
      return {
        headers,
      };
    }
  }
}
