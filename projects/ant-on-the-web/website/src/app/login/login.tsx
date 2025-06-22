import { login } from "@/server/posts";
import { useContext, useState } from "react";
import { UserContext } from "../../state/userContext";
import { useRouter } from "next/navigation";
import { getUser, getUserSchema } from "@/server/queries";
import Link from "next/link";

export const LoginBox = () => {
  const [loginUnique, setLoginUnique] = useState("");
  const [loginValidationMsg, setLoginValidationMsg] = useState("");

  const [passwordAttempt, setPasswordAttempt] = useState("");
  const [passwordAttemptValidationMsg, setPasswordAttemptValidationMsg] =
    useState("");

  const { setUser } = useContext(UserContext);

  const [formState, setFormState] = useState<
    { loading: false; success: boolean; msg: string } | { loading: true }
  >({ loading: false, success: false, msg: "" });

  async function handleLogin(e: any) {
    e.preventDefault();
    setFormState({ loading: true });
    const response = await login({
      method: { username: loginUnique },
      password: passwordAttempt,
    });

    switch (response.status) {
      case 500: {
        return setFormState({
          loading: false,
          success: false,
          msg: "something went wrong, please retry.",
        });
      }

      case 401: {
        return setFormState({
          loading: false,
          success: false,
          msg: "username or password invalid.",
        });
      }

      case 400: {
        const resp: { errors: { field: string; msg: string }[] } =
          await response.json();

        for (const error of resp.errors) {
          switch (error.field) {
            case "method.email":
            case "method.username":
            case "method.phone": {
              setLoginValidationMsg(error.msg.toLowerCase());
              break;
            }
            case "password": {
              setPasswordAttemptValidationMsg(error.msg.toLowerCase());
              break;
            }
            default: {
              setFormState({
                loading: false,
                success: false,
                msg: "invalid field, please retry.",
              });
              break;
            }
          }
        }
      }

      case 200: {
        const body: { userId: string } = await response.json();
        setLoginUnique("");
        setLoginValidationMsg("");

        setPasswordAttempt("");
        setPasswordAttemptValidationMsg("");

        setFormState({
          loading: false,
          success: true,
          msg: "login complete, welcome!",
        });
        setUser({ weakAuth: true, loggedIn: false });
        return;
      }
    }
  }

  return (
    <div>
      <div className="mb-2">if you already have an account, login:</div>
      <form autoComplete="off" onSubmit={(event) => handleLogin(event)}>
        <div className="grid grid-cols-3 gap-0">
          <span className="flex flex-col justify-center">your username:</span>
          <input
            className="m-1"
            type="text"
            name="unique"
            autoComplete="off"
            placeholder="ex. kaspar"
            value={loginUnique}
            onChange={(e) => {
              setLoginUnique(e.target.value);
              setLoginValidationMsg("");
            }}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {loginValidationMsg}
          </span>
          <span className="flex flex-col justify-center">your password:</span>
          <input
            className="m-1"
            type="password"
            name="password"
            autoComplete="off"
            value={passwordAttempt}
            onChange={(e) => {
              setPasswordAttempt(e.target.value);
              setPasswordAttemptValidationMsg("");
            }}
          />
          <span className="flex flex-col justify-center m-1 text-red-600 content-center">
            {passwordAttemptValidationMsg}
          </span>
        </div>

        <div className="flex flex-row justify-start w-8/12">
          <Link href="/login/forgot-password">forgot your password?</Link>
        </div>
        <div className="flex flex-row justify-center w-8/12">
          <input type="submit" className="w-full m-1" value="login" />
        </div>
        <span
          className={`m-1 text-${
            formState.loading ? "blue" : formState.success ? "green" : "red"
          }-600 content-center`}
        >
          {formState.loading ? "loading..." : formState.msg}
        </span>
      </form>
    </div>
  );
};
