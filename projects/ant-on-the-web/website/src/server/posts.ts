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
      password: z.string(),
      password2: z.string(),
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
  changeUsername: {
    path: "/api/users/username",
    inputDataSchema: z.object({
      username: z.string(),
    }),
  },
  addPhoneNumber: {
    path: "/api/users/phone-number",
    inputDataSchema: z.object({
      phoneNumber: z.string(),
      forceSend: z.boolean(),
    }),
  },
  addEmail: {
    path: "/api/users/email",
    inputDataSchema: z.object({
      email: z.string(),
      forceSend: z.boolean(),
    }),
  },
  verificationAttempt: {
    path: "/api/users/verification-attempt",
    inputDataSchema: z.object({
      method: z.union([
        z.object({
          email: z.object({
            email: z.string(),
            otp: z.string(),
          }),
        }),
        z.object({
          phone: z.object({
            phoneNumber: z.string(),
            otp: z.string(),
          }),
        }),
      ]),
    }),
  },
  passwordResetCode: {
    path: "/api/users/password-reset-code",
    inputDataSchema: z.object({
      username: z.string(),
      phoneNumber: z.string(),
    }),
  },
  passwordResetSecret: {
    path: "/api/users/password-reset-secret",
    inputDataSchema: z.object({
      phoneNumber: z.string(),
      otp: z.string(),
    }),
  },
  password: {
    path: "/api/users/password",
    inputDataSchema: z.object({
      secret: z.string(),
      password1: z.string(),
      password2: z.string(),
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
): Promise<Response> {
  const { path, inputDataSchema } = query;

  const input = inputDataSchema.parse(inputData);
  const endpoint = getEndpoint(path);

  console.log("POST: ", endpoint.toString());

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
export const changeUsername = (
  inputData: z.infer<typeof posts.changeUsername.inputDataSchema>
) => constructPost(posts.changeUsername, inputData);
export const addPhoneNumber = (
  inputData: z.infer<typeof posts.addPhoneNumber.inputDataSchema>
) => constructPost(posts.addPhoneNumber, inputData);
export const addEmail = (
  inputData: z.infer<typeof posts.addEmail.inputDataSchema>
) => constructPost(posts.addEmail, inputData);
export const verificationAttempt = (
  inputData: z.infer<typeof posts.verificationAttempt.inputDataSchema>
) => constructPost(posts.verificationAttempt, inputData);
export const passwordResetCode = (
  inputData: z.infer<typeof posts.passwordResetCode.inputDataSchema>
) => constructPost(posts.passwordResetCode, inputData);
export const passwordResetSecret = (
  inputData: z.infer<typeof posts.passwordResetSecret.inputDataSchema>
) => constructPost(posts.passwordResetSecret, inputData);
export const password = (
  inputData: z.infer<typeof posts.password.inputDataSchema>
) => constructPost(posts.password, inputData);
