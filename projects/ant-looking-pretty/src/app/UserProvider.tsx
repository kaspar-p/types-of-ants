"use client";

import { createContext, use, useContext } from "react";
import { PropsWithChildren } from "react";

export const useUser = (): {
  user: TUserContext;
  resetUser: () => void;
} => {
  const user = useContext(UserContext);
  if (!user) throw new Error("UserContext not initialized");

  return {
    user: use(user.user),
    resetUser: user.resetUser,
  };
};

export type User = {
  userId: string;
  username: string;
  phoneNumbers: string[];
  emails: string[];
  joined: Date;
};

export type TUserContext = { loggedIn: false } | { loggedIn: true; user: User };

export const UserContext = createContext<
  undefined | { user: Promise<TUserContext>; resetUser: () => void }
>(undefined);

export function UserProvider(
  props: PropsWithChildren<{
    user: Promise<TUserContext>;
    resetUser: () => void;
  }>,
) {
  return (
    <UserContext.Provider
      value={{ user: props.user, resetUser: props.resetUser }}
    >
      {props.children}
    </UserContext.Provider>
  );
}
