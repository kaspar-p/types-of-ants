import { TUserContext, User } from "@/app/UserProvider";
import { getUser, getUserSchema } from "@/server/queries";
import { cache } from "react";

export const getAuth = cache(async (): Promise<TUserContext> => {
  const u = await getUser();
  if (u.ok) {
    const res: unknown = await u.json();
    const user = getUserSchema.transformer(getUserSchema.schema.parse(res));
    return { loggedIn: true, user: user.user };
  }
  console.log("no user");
  return { loggedIn: false };
});
