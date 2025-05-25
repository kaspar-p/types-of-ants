import { createContext } from "react";

export type User = {
  userId: string;
  username: string;
  phoneNumber: string;
  emails: string[];
  joined: Date;
};
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
