import { login } from "@/server/posts";
import { useState } from "react";

export const LoginBox = () => {
  const [loginUnique, setLoginUnique] = useState("");
  const [loginValidationMsg, setLoginValidationMsg] = useState("");

  const [passwordAttempt, setPasswordAttempt] = useState("");
  const [passwordAttemptValidationMsg, setPasswordAttemptValidationMsg] =
    useState("");

  const [formState, setFormState] = useState<{ success: boolean; msg: string }>(
    { success: false, msg: "" }
  );

  async function handleLogin(e: any) {
    e.preventDefault();
    const response = await login({
      method: { username: loginUnique },
      password: passwordAttempt,
    });

    switch (response.status) {
      case 500: {
        return setFormState({
          success: false,
          msg: "something went wrong, please retry.",
        });
      }

      case 401: {
        return setFormState({
          success: false,
          msg: "username or password invalid.",
        });
      }

      case 400: {
        const j: { field: string; msg: string } = await response.json();
        switch (j.field) {
          case "method":
            return setLoginValidationMsg(j.msg.toLowerCase());
          case "password":
            return setPasswordAttemptValidationMsg(j.msg.toLowerCase());
          default:
            return setFormState({
              success: false,
              msg: "invalid field, please retry.",
            });
        }
      }

      case 200: {
        setLoginUnique("");
        setLoginValidationMsg("");

        setPasswordAttempt("");
        setPasswordAttemptValidationMsg("");

        setFormState({ success: true, msg: "login complete, welcome!" });
        return;
      }
    }
  }

  return (
    <form
      className="grid grid-cols-3 max-w-xl gap-0"
      autoComplete="off"
      onSubmit={(event) => handleLogin(event)}
    >
      <span>your username:</span>
      <input
        className="m-1"
        type="text"
        name="unique"
        autoComplete="off"
        placeholder="ex. kaspar"
        value={loginUnique}
        onChange={(e) => setLoginUnique(e.target.value)}
      />
      <span className={`m-1 text-red-600 content-center`}>
        {loginValidationMsg}
      </span>
      <span>your password:</span>
      <input
        className="m-1"
        type="password"
        name="password"
        autoComplete="off"
        value={passwordAttempt}
        onChange={(e) => setPasswordAttempt(e.target.value)}
      />
      <span className="text-red-600">{passwordAttemptValidationMsg}</span>
      <input type="submit" className="m-1" value="login" />
      <span
        className={`m-1 text-${
          formState.success ? "green" : "red"
        }-600 content-center`}
      >
        {formState.msg}
      </span>
    </form>
  );
};
