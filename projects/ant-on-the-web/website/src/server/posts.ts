import { z } from "zod";
import { getEndpoint, getFetchOptions } from "./lib";

const posts = {
  suggestAnt: {
    path: "/api/ants/suggest",
    inputDataSchema: z.object({
      suggestionContent: z.string(),
    }),
  },
  signup: {
    path: "/api/users/signup",
    inputDataSchema: z.object({
      username: z.string(),
      email: z.string(),
      phoneNumber: z.string(),
      password: z.string(),
    }),
  },
  login: {
    path: "/api/users/login",
    inputDataSchema: z.object({
      method: z.union([
        z.object({ username: z.string() }),
        z.object({ phoneNumber: z.string() }),
        z.object({ email: z.string() }),
      ]),
      password: z.string(),
    }),
  },
  logout: {
    path: "/api/users/logout",
    inputDataSchema: z.object({}),
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
): Promise<Response> {
  const { path, inputDataSchema } = query;
  console.log("POST: ", query.path);

  const input = inputDataSchema.parse(inputData);
  const endpoint = getEndpoint(path);
  return await fetch(endpoint, {
    method: "POST",
    headers: {
      "Content-type": "application/json",
    },
    body: JSON.stringify(input),
    ...getFetchOptions(),
  });
}

export const suggestAnt = (
  inputData: z.infer<typeof posts.suggestAnt.inputDataSchema>
) => constructPost(posts.suggestAnt, inputData);
export const newsletterSignup = (
  inputData: z.infer<typeof posts.newsletterSignup.inputDataSchema>
) => constructPost(posts.newsletterSignup, inputData);
export const signup = (
  inputData: z.infer<typeof posts.signup.inputDataSchema>
) => constructPost(posts.signup, inputData);
export const logout = () => constructPost(posts.logout, {});
export const login = (inputData: z.infer<typeof posts.login.inputDataSchema>) =>
  constructPost(posts.login, inputData);
