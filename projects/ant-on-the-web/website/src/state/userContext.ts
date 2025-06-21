import { createContext } from "react";

export type User = {
  userId: string;
  username: string;
  phoneNumbers: string[];
  emails: string[];
  joined: Date;
};
export type TUserContext =
  | { weakAuth: false }
  | { weakAuth: true; loggedIn: false }
  | { weakAuth: true; loggedIn: true; user: User };

export const UserContext = createContext<{
  user: TUserContext;
  setUser: (user: TUserContext) => void;
}>({
  user: {
    weakAuth: false,
  },
  setUser: () => {},
});
