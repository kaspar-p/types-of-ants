import { Response } from "@/utils/useQuery";
import { z } from "zod";

type QueryMetadata = {
  name: string;
  endpoint: string;
  schema: z.ZodSchema extends z.ZodSchema<infer T> ? z.ZodSchema<T> : never;
  transformer: (data: any) => any;
};

const queries = {
  getReleaseNumber: {
    name: "getReleaseNumber",
    endpoint: "/api/ants/latest-release",
    schema: z.number(),
    transformer: (data: number) => data,
  },
  getLatestAnts: {
    name: "getLatestAnts",
    endpoint: "/api/ants/latest-ants",
    schema: z.object({
      date: z.number(),
      ants: z.array(z.object({ ant_id: z.string(), ant_name: z.string() })),
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
  getAllAnts: {
    name: "getAllAnts",
    endpoint: "/api/ants/all-ants",
    schema: z.object({
      ants: z.array(z.object({ ant_id: z.string(), ant_name: z.string() })),
    }),
    transformer: (data: {
      ants: { ant_id: string; ant_name: string }[];
    }): { ants: string[] } => ({
      ants: data.ants.map((ant) => ant.ant_name),
    }),
  },
} as const;

type Query = (typeof queries)[keyof typeof queries];
type QueryRet<Q extends Query> = ReturnType<Q["transformer"]>;

async function constructQuery<Q extends Query>(
  query: Q
): Promise<Response<QueryRet<Q>>> {
  const { endpoint, schema, transformer } = query;
  console.log("GET: ", query.endpoint);

  const response = await fetch(`http://localhost:3499${endpoint}`);
  const rawData = await response.json();
  console.log("GOT DATA: ", rawData, "AND RESPONSE", response);
  if (response.status >= 300) return { success: false };
  const result = schema.safeParse(rawData);
  if (!result.success) {
    console.log("FAILED", result.error);
    return { success: false };
  }

  const data = result.data as any;

  return {
    success: true,
    data: transformer(data) as any,
  };
}

export const getLatestAnts = () => constructQuery(queries.getLatestAnts);
export const getAllAnts = () => constructQuery(queries.getAllAnts);
export const getReleaseNumber = () => constructQuery(queries.getReleaseNumber);
