import { z } from "zod";

const posts = {
  suggestAnt: {
    endpoint: "/api/ants/suggest",
    inputDataSchema: z.object({
      suggestion_content: z.string(),
    }),
  },
};

type Query = (typeof posts)[keyof typeof posts];

async function constructPost<Q extends Query>(
  query: Q,
  inputData: Q["inputDataSchema"]
): Promise<{ success: boolean }> {
  const { endpoint, inputDataSchema } = query;

  const input = inputDataSchema.parse(inputData);
  const response = await fetch(`http://localhost:3499${endpoint}`, {
    method: "POST",
    body: JSON.stringify(input),
  });
  const rawData = await response.json();
  console.log("GOT RESPONSE: ", rawData);
  if (response.status >= 300) return { success: false };
  return {
    success: true,
  };
}

export const suggestAnt = (
  inputData: typeof posts.suggestAnt.inputDataSchema
) => constructPost(posts.suggestAnt, inputData);
