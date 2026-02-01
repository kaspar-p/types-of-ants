export type AntOnTheWebError =
  | {
      __status: 400;
      __type: "ValidationError";
      errors: { field?: string; msg: string }[];
    }
  | { __status: 409; __type: "ConflictException"; msg: string }
  | {
      __status: 500;
      __type: "InternalServerError";
      msg: "something went wrong, please retry.";
    }
  | { __status: 401; __type: "AccessDenied"; msg: "access denied." };

export type Release = {
  releaseNumber: number;
  releaseLabel: string;
  createdAt: number;
  createdBy: string;
};

export type AntOnTheWebResponse = { __status: 200 } & (
  | { __type: "LatestReleaseResponse"; release: Release }
  | { __type: "SuggestionResponse" }
  | { __type: "SignupResponse" }
  | { __type: "LoginResponse" }
  | { __type: "LogoutResponse" }
  | { __type: "ChangeUsernameResponse" }
  | { __type: "AddPhoneNumberResponse" }
  | { __type: "AddEmailResponse" }
  | { __type: "VerificationAttemptResponse" }
  | { __type: "PasswordResetCodeResponse" }
  | { __type: "PasswordResetSecretResponse"; secret: string }
  | { __type: "PasswordResetResponse" }
  | { __type: "SubscribeNewsletterResponse" }
  | { __type: "WebActionResponse" }
  | { __type: "FavoriteAntResponse"; favoritedAt: string }
  | { __type: "UnfavoriteAntResponse" }
);

export type QueryResponse<T extends AntOnTheWebResponse["__type"]> =
  | (AntOnTheWebResponse & { __type: T })
  | AntOnTheWebError;
