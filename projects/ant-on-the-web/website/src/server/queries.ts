import { z } from "zod";
import { getEndpoint, getFetchOptions } from "./lib";

const antSchema = z.object({
  antId: z.string(),
  hash: z.number().optional(),
  antName: z.string(),
  createdAt: z.string(),
  createdByUsername: z.string(),
});
export type Ant = z.infer<typeof antSchema>;
export type Ants = Ant[];

const contentHash = async (content: string): Promise<number> => {
  const encoder = new TextEncoder();
  const buf = await crypto.subtle.digest("SHA-512", encoder.encode(content));
  const view = new DataView(buf);
  const hash = Math.abs(view.getInt32(0, false));
  console.log(content, hash);
  return hash;
};

const queries = {
  getVersion: {
    name: "getVersion",
    path: "/api/version",
    schema: z.string(),
    transformer: (d: string): string => d,
    isJson: false,
  },
  getLatestRelease: {
    name: "getLatestRelease",
    path: "/api/ants/latest-release",
    schema: z.object({
      release: z.object({
        releaseNumber: z.number(),
        createdAt: z.string(),
      }),
    }),
    transformer: (data: {
      release: {
        releaseNumber: number;
        createdAt: string;
      };
    }): { release: { releaseNumber: number; createdAt: Date } } => {
      return {
        release: {
          releaseNumber: data.release.releaseNumber,
          createdAt: new Date(data.release.createdAt),
        },
      };
    },
  },
  getLatestAnts: {
    name: "getLatestAnts",
    path: "/api/ants/latest-ants",
    schema: z.object({
      date: z.number(),
      ants: z.array(antSchema),
    }),
    transformer: (data: {
      date: number;
      ants: { antId: string; antName: string }[];
    }): { date: Date; ants: string[] } => {
      return {
        date: new Date(data.date * 1000),
        ants: data.ants.map((ant) => ant.antName),
      };
    },
  },
  getTotalAnts: {
    name: "getTotalAnts",
    path: "/api/ants/total",
    schema: z.object({
      total: z.number(),
    }),
    transformer(d: { total: number }): number {
      return d.total;
    },
  },
  getUnseenAnts: {
    name: "getUnseenAntsPaginated",
    path: "/api/ants/unreleased-ants",
    queryParams: ["page"],
    schema: z.object({
      ants: z.array(antSchema),
    }),
    transformer: (data: { ants: Ants }): Ants => {
      return data.ants;
    },
  },
  getReleasedAnts: {
    name: "getReleasedAnts",
    path: "/api/ants/released-ants",
    queryParams: ["page"],
    schema: z.object({
      ants: z.array(antSchema),
      hasNextPage: z.boolean(),
    }),
    transformer: async (data: {
      ants: Ants;
      hasNextPage: boolean;
    }): Promise<{ ants: string[]; hasNextPage: boolean }> => {
      let ants = await Promise.all(
        data.ants.map(async (a) => ({
          ...a,
          hash: a.hash ?? (await contentHash(a.antName)),
        }))
      );

      ants.sort((a, b) => (a.hash < b.hash ? -1 : 1));

      return {
        ants: ants.map((a) => a.antName),
        hasNextPage: data.hasNextPage,
      };
    },
  },
  getUser: {
    name: "getUser",
    path: "/api/users/user",
    schema: z.object({
      user: z.object({
        userId: z.string(),
        username: z.string(),
        emails: z.array(z.string()),
        joined: z.number(),
        phoneNumbers: z.array(z.string()),
      }),
    }),
    transformer: (d: {
      user: {
        userId: string;
        phoneNumbers: string[];
        emails: string[];
        joined: number;
        username: string;
      };
    }): {
      user: {
        userId: string;
        phoneNumbers: string[];
        emails: string[];
        joined: Date;
        username: string;
      };
    } => ({
      ...d,
      user: { ...d.user, joined: new Date(d.user.joined * 1000) },
    }),
  },
  getPasswordResetCode: {
    name: "getPasswordResetCode",
    path: "/api/users/password-reset-code",
    transformer: (d: any) => d,
    inputSchema: z.object({
      username: z.string(),
      phoneNumber: z.string(),
    }),
  },
} as const;

type Query = (typeof queries)[keyof typeof queries];
type QueryRet<Q extends Query> = ReturnType<Q["transformer"]>;
type QueryParams<Q extends Query> = Q extends { queryParams: any }
  ? { [x in Q["queryParams"][number]]: unknown }
  : undefined;

async function constructQuery<Q extends Query>(
  query: Q,
  inputData?: QueryParams<Q>
): Promise<Awaited<ReturnType<Q["transformer"]>>> {
  const endpoint = getEndpoint(query.path);
  if ("queryParams" in query && inputData !== undefined) {
    for (const param of query.queryParams) {
      endpoint.searchParams.set(
        param,
        encodeURIComponent(JSON.stringify(inputData[param]))
      );
    }
  }

  console.log("GET: ", endpoint.toString());

  const res = await fetch(endpoint, getFetchOptions());

  let data: any;
  if ("isJson" in query && !query.isJson) {
    data = await res.text();
  } else {
    data = await res.json();
  }

  const transformedData = await query.transformer(data);
  return transformedData as any as Awaited<QueryRet<Q>>;
}

export const getVersion = () => constructQuery(queries.getVersion);
export const getLatestAnts = () => constructQuery(queries.getLatestAnts);
export const getTotalAnts = () => constructQuery(queries.getTotalAnts);
export const getReleasedAnts = (page: number) =>
  constructQuery(queries.getReleasedAnts, { page });
export const getUnseenAnts = (page: number) =>
  constructQuery(queries.getUnseenAnts, { page });
export const getLatestRelease = () => constructQuery(queries.getLatestRelease);

async function constructQuery2<Q extends Query>(
  query: Q,
  inputData?: QueryParams<Q>
): Promise<Response> {
  const endpoint = getEndpoint(query.path);
  if ("queryParams" in query && inputData !== undefined) {
    for (const param of query.queryParams) {
      endpoint.searchParams.set(
        param,
        encodeURIComponent(JSON.stringify(inputData[param]))
      );
    }
  }

  console.log("GET: ", endpoint.toString());
  const response = await fetch(endpoint, getFetchOptions());

  return response;
}

export const getUser = () => constructQuery2(queries.getUser);
export const getUserSchema = queries.getUser;
