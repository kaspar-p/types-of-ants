import { z } from "zod";
import { getEndpoint } from "./lib";

const posts = {
  suggestAnt: {
    path: "/api/ants/suggest",
    inputDataSchema: z.object({
      suggestion_content: z.string(),
    }),
  },
  newsletterSignup: {
    path: "/api/users/subscribe-newsletter",
    inputDataSchema: z.object({
      email: z.string(),
    }),
  },
};

type Query = (typeof posts)[keyof typeof posts];

async function constructPost<Q extends Query>(
  query: Q,
  inputData: z.infer<Q["inputDataSchema"]>
): Promise<{ success: boolean }> {
  const { path, inputDataSchema } = query;
  console.log("POST: ", query.path);

  const input = inputDataSchema.parse(inputData);
  const endpoint = getEndpoint(path);
  const response = await fetch(endpoint, {
    method: "POST",
    headers: {
      "Content-type": "application/json",
    },
    body: JSON.stringify(input),
  });
  console.log("GOT RESPONSE: ", response);
  try {
    await response.json();
  } catch (e) {
    console.error(e);
    throw e;
  }

  if (response.status >= 300) return { success: false };
  return {
    success: true,
  };
}

export const suggestAnt = (
  inputData: z.infer<typeof posts.suggestAnt.inputDataSchema>
) => constructPost(posts.suggestAnt, inputData);
export const newsletterSignup = (
  inputData: z.infer<typeof posts.newsletterSignup.inputDataSchema>
) => constructPost(posts.newsletterSignup, inputData);
