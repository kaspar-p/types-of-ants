import { z } from "zod";

const posts = {
  suggestAnt: {
    endpoint: "/api/ants/suggest",
    inputDataSchema: z.object({
      suggestion_content: z.string(),
    }),
  },
  newsletterSignup: {
    endpoint: "/api/users/subscribe-newsletter",
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
  const { endpoint, inputDataSchema } = query;
  console.log("POST: ", query.endpoint);

  const input = inputDataSchema.parse(inputData);
  const response = await fetch(`http://localhost:3499${endpoint}`, {
    method: "POST",
    headers: {
      "Content-type": "application/json",
    },
    body: JSON.stringify(input),
  });
  console.log("GOT RESPONSE: ", response);
  try {
    const rawData = await response.json();
  } catch (e) {
    console.error(e);
    throw e;
  }
  console.log("GOT DATA: ", {});
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
