import { z } from "zod";
import { getEndpoint, getFetchOptions } from "./lib";

const antSchema = z.object({
  ant_id: z.string(),
  ant_name: z.string(),
  created_at: z.string(),
});
export type Ant = z.infer<typeof antSchema>;
export type Ants = Ant[];

const queries = {
  getLatestRelease: {
    name: "getLatestRelease",
    path: "/api/ants/latest-release",
    schema: z.object({
      release_number: z.number(),
      created_at: z.string(),
    }),
    transformer: (data: {
      release_number: number;
      created_at: string;
    }): { release_number: number; created_at: Date } => {
      return {
        release_number: data.release_number,
        created_at: new Date(data.created_at),
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
      ants: { ant_id: string; ant_name: string }[];
    }): { date: Date; ants: string[] } => {
      return {
        date: new Date(data.date * 1000),
        ants: data.ants.map((ant) => ant.ant_name),
      };
    },
  },
  getUnseenAnts: {
    name: "getUnseenAntsPaginated",
    path: "/api/ants/unreleased-ants",
    queryParams: ["page"],
    schema: z.object({ ants: z.array(antSchema) }),
    transformer: (data: Ants): Ants => {
      return data;
    },
  },
  getReleasedAnts: {
    name: "getReleasedAnts",
    path: "/api/ants/released-ants",
    queryParams: ["page"],
    schema: z.object({ ants: z.array(antSchema) }),
    transformer: (data: Ants): { ants: string[] } => ({
      ants: data.map((ant) => ant.ant_name),
    }),
  },
  getUser: {
    name: "getUser",
    path: "/api/users/user",
    schema: z.object({
      user: z.object({
        userId: z.string(),
      }),
    }),
    transformer: (d: { user: { userId: string } }) => d,
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
): Promise<ReturnType<Q["transformer"]>> {
  const endpoint = getEndpoint(query.path);
  if ("queryParams" in query && inputData !== undefined) {
    for (const param of query.queryParams) {
      endpoint.searchParams.set(
        param,
        encodeURIComponent(JSON.stringify(inputData[param]))
      );
    }
  }
  console.log("GET: ", endpoint);

  const data = await (await fetch(endpoint)).json();
  console.log("GOT DATA: ", data);
  const transformedData = query.transformer(data);
  return transformedData as any as QueryRet<Q>;
}

export const getLatestAnts = () => constructQuery(queries.getLatestAnts);
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
  console.log("GET: ", endpoint);

  const response = await fetch(endpoint, getFetchOptions());

  return response;
}
export const getUser = () => constructQuery2(queries.getUser);
