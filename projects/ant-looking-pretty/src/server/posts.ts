import { z } from "zod";
import { getEndpoint, getFetchOptions } from "./lib";
import { QueryResponse } from "./rpc";

const posts = {
  suggestAnt: {
    responseType: "SuggestionResponse",
    path: "/api/ants/suggest",
    inputDataSchema: z.object({
      suggestionContent: z.string(),
    }),
  },
  signup: {
    responseType: "SignupResponse",
    path: "/api/users/signup",
    inputDataSchema: z.object({
      username: z.string(),
      password: z.string(),
      password2: z.string(),
    }),
  },
  login: {
    responseType: "LoginResponse",
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
    responseType: "LogoutResponse",
    path: "/api/users/logout",
    inputDataSchema: z.object({}),
  },
  changeUsername: {
    responseType: "ChangeUsernameResponse",
    path: "/api/users/username",
    inputDataSchema: z.object({
      username: z.string(),
    }),
  },
  addPhoneNumber: {
    responseType: "AddPhoneNumberResponse",
    path: "/api/users/phone-number",
    inputDataSchema: z.object({
      phoneNumber: z.string(),
      forceSend: z.boolean(),
    }),
  },
  addEmail: {
    responseType: "AddEmailResponse",
    path: "/api/users/email",
    inputDataSchema: z.object({
      email: z.string(),
      forceSend: z.boolean(),
    }),
  },
  verificationAttempt: {
    responseType: "VerificationAttemptResponse",
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
    responseType: "PasswordResetCodeResponse",
    path: "/api/users/password-reset-code",
    inputDataSchema: z.object({
      username: z.string(),
      phoneNumber: z.string(),
    }),
  },
  passwordResetSecret: {
    responseType: "PasswordResetSecretResponse",
    path: "/api/users/password-reset-secret",
    inputDataSchema: z.object({
      phoneNumber: z.string(),
      otp: z.string(),
    }),
  },
  password: {
    responseType: "PasswordResetResponse" as const,
    path: "/api/users/password",
    inputDataSchema: z.object({
      secret: z.string(),
      password1: z.string(),
      password2: z.string(),
    }),
  },
  newsletterSignup: {
    responseType: "SubscribeNewsletterResponse" as const,
    path: "/api/users/subscribe-newsletter",
    inputDataSchema: z.object({
      email: z.string(),
    }),
  },
  webAction: {
    responseType: "WebActionResponse" as const,
    path: "/api/web-actions/action",
    inputDataSchema: z.object({
      action: z.union([
        z.literal("visit"),
        z.literal("click"),
        z.literal("hover"),
      ]),
      targetType: z.union([z.literal("page"), z.literal("button")]),
      target: z.string(),
    }),
  },
  favorite: {
    responseType: "FavoriteAntResponse" as const,
    path: "/api/ants/favorite",
    inputDataSchema: z.object({
      antId: z.string(),
    }),
  },
  unfavorite: {
    responseType: "UnfavoriteAntResponse" as const,
    path: "/api/ants/unfavorite",
    inputDataSchema: z.object({
      antId: z.string(),
    }),
  },
} as const;

type Query = (typeof posts)[keyof typeof posts];

async function constructPost<Q extends Query>(
  query: Q,
  inputData: z.infer<Q["inputDataSchema"]>,
): Promise<QueryResponse<Q["responseType"]>> {
  const { responseType, path, inputDataSchema } = query;

  const input = inputDataSchema.parse(inputData);
  const endpoint = getEndpoint(path);

  console.log("POST: ", endpoint.toString());

  const opts = await getFetchOptions();
  const headers = {
    ...(opts.headers ?? {}),
    "Content-Type": "application/json",
  };
  const res = await fetch(endpoint, {
    ...opts,
    cache: "no-store",
    method: "POST",
    headers: headers,
    body: JSON.stringify(input),
  });

  const body = await res.json();

  if (res.ok && body.__type !== responseType) {
    throw new Error(
      `Expected response __type=${responseType} but received ${body.__type}`,
    );
  }

  return {
    __status: res.status,
    ...body,
  };
}

export const suggestAnt = async (
  inputData: z.infer<typeof posts.suggestAnt.inputDataSchema>,
) => await constructPost(posts.suggestAnt, inputData);
export const newsletterSignup = async (
  inputData: z.infer<typeof posts.newsletterSignup.inputDataSchema>,
) => await constructPost(posts.newsletterSignup, inputData);
export const signup = async (
  inputData: z.infer<typeof posts.signup.inputDataSchema>,
) => await constructPost(posts.signup, inputData);
export const logout = async () => await constructPost(posts.logout, {});
export const login = async (
  inputData: z.infer<typeof posts.login.inputDataSchema>,
) => constructPost(posts.login, inputData);
export const changeUsername = async (
  inputData: z.infer<typeof posts.changeUsername.inputDataSchema>,
) => await constructPost(posts.changeUsername, inputData);
export const addPhoneNumber = async (
  inputData: z.infer<typeof posts.addPhoneNumber.inputDataSchema>,
) => await constructPost(posts.addPhoneNumber, inputData);
export const addEmail = async (
  inputData: z.infer<typeof posts.addEmail.inputDataSchema>,
) => await constructPost(posts.addEmail, inputData);
export const verificationAttempt = async (
  inputData: z.infer<typeof posts.verificationAttempt.inputDataSchema>,
) => await constructPost(posts.verificationAttempt, inputData);
export const passwordResetCode = async (
  inputData: z.infer<typeof posts.passwordResetCode.inputDataSchema>,
) => await constructPost(posts.passwordResetCode, inputData);
export const passwordResetSecret = async (
  inputData: z.infer<typeof posts.passwordResetSecret.inputDataSchema>,
) => await constructPost(posts.passwordResetSecret, inputData);
export const password = async (
  inputData: z.infer<typeof posts.password.inputDataSchema>,
) => await constructPost(posts.password, inputData);
export const webAction = async (
  inputData: z.infer<typeof posts.webAction.inputDataSchema>,
) => await constructPost(posts.webAction, inputData);
export const favorite = async (
  inputData: z.infer<typeof posts.favorite.inputDataSchema>,
) => await constructPost(posts.favorite, inputData);
export const unfavorite = async (
  inputData: z.infer<typeof posts.unfavorite.inputDataSchema>,
) => await constructPost(posts.unfavorite, inputData);
