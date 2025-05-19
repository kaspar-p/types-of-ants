import { createContext } from "react";

export type User = { userId: string; username: string };
export type TUserContext = { loggedIn: false } | { loggedIn: true; user: User };

export const UserContext = createContext<{
  user: TUserContext;
  setUser: (user: TUserContext) => void;
}>({
  user: {
    loggedIn: false,
  },
  setUser: () => {},
});
