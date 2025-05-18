import { signup } from "@/server/posts";
import { useState } from "react";

export function SignupBox() {
  const [formMsg, setFormMsg] = useState("");
  const [formColor, setFormColor] = useState<"red" | "green">("red");

  const [username, setUsername] = useState("");
  const [usernameValidationMsg, setUsernameValidationMsg] = useState("");

  const [phone, setPhone] = useState("");
  const [phoneValidationMsg, setPhoneValidationMsg] = useState("");

  const [email, setEmail] = useState("");
  const [emailValidationMsg, setEmailValidationMsg] = useState("");

  const [password, setPassword] = useState("");
  const [passwordValidationMsg, setPasswordValidationMsg] = useState("");

  async function handle(e: any) {
    e.preventDefault();
    console.log(": ", e.target.value, username, phone, email);

    const response = await signup({
      username: username,
      phoneNumber: phone,
      email: email,
      password: password,
    });

    switch (response.status) {
      default:
      case 500: {
        return setFormMsg("something went wrong, please retry.");
      }

      case 409: {
        const j: { msg: string } = await response.json();
        return setFormMsg(j.msg.toLowerCase());
      }

      case 400: {
        const j: { field: string; msg: string } = await response.json();
        switch (j.field) {
          case "phoneNumber":
            return setPhoneValidationMsg(j.msg.toLowerCase());
          case "email":
            return setEmailValidationMsg(j.msg.toLowerCase());
          case "username":
            return setUsernameValidationMsg(j.msg.toLowerCase());
          case "password":
            return setPasswordValidationMsg(j.msg.toLowerCase());
          default:
            return setFormMsg("invalid field, please retry.");
        }
      }

      case 200: {
        const j = await response.text();

        setPhone("");
        setPhoneValidationMsg("");
        setUsername("");
        setUsernameValidationMsg("");
        setEmail("");
        setEmailValidationMsg("");
        setPassword("");
        setPasswordValidationMsg("");

        setFormColor("green");
        setFormMsg("signup complete, welcome!");
      }
    }
  }

  return (
    <div>
      <div className="mb-2">if you don&apos;t have an account, signup:</div>
      <form
        className="grid grid-cols-3 max-w-xl gap-0"
        autoComplete="off"
        onSubmit={(event) => handle(event)}
      >
        <span>your username: </span>
        <input
          className="m-1"
          type="text"
          name="username"
          autoComplete="off"
          placeholder="ex. kaspar"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
        />
        <span className={`m-1 text-red-600 content-center`}>
          {usernameValidationMsg}
        </span>

        <span>your phone number: </span>
        <input
          className="m-1"
          type="text"
          name="phoneNumber"
          autoComplete="off"
          placeholder="ex. +1 (000) 111-2222"
          value={phone}
          onChange={(e) => setPhone(e.target.value)}
        />
        <span className={`m-1 text-red-600 content-center`}>
          {phoneValidationMsg}
        </span>

        <span>your email: </span>
        <input
          className="m-1"
          type="text"
          name="email"
          autoComplete="off"
          placeholder="ex. kaspar@typesofants.org"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
        />
        <span className={`m-1 text-red-600 content-center`}>
          {emailValidationMsg}
        </span>

        <span>your password: </span>
        <input
          className="m-1"
          type="password"
          autoComplete="off"
          name="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
        <span className={`m-1 text-red-600 content-center`}>
          {passwordValidationMsg}
        </span>

        <input type="submit" className="m-1" value="signup" />
        <span className={`m-1 text-${formColor}-600 content-center`}>
          {formMsg}
        </span>
      </form>
    </div>
  );
}
