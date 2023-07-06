import { z } from "zod";
import { getEndpoint } from "./lib";

const antsSchema = z.array(
  z.object({ ant_id: z.string(), ant_name: z.string(), created_at: z.string() })
);
export type Ants = z.infer<typeof antsSchema>;

const queries = {
  getReleaseNumber: {
    name: "getReleaseNumber",
    path: "/api/ants/latest-release",
    schema: z.number(),
    transformer: (data: number): number => {
      return data;
    },
  },
  getLatestAnts: {
    name: "getLatestAnts",
    path: "/api/ants/latest-ants",
    schema: z.object({
      date: z.number(),
      ants: antsSchema,
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
    schema: z.object({ ants: antsSchema }),
    transformer: (data: Ants): Ants => {
      return data;
    },
  },
  getReleasedAnts: {
    name: "getReleasedAnts",
    path: "/api/ants/released-ants",
    queryParams: ["page"],
    schema: z.object({ ants: antsSchema }),
    transformer: (data: Ants): { ants: string[] } => ({
      ants: data.map((ant) => ant.ant_name),
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
export const getReleaseNumber = () => constructQuery(queries.getReleaseNumber);
