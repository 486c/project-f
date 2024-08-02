import { useState } from "preact/hooks";
import { useNavigate } from "react-router-dom";

export function SetTokenPage() {
  const [token, setToken] = useState<string>("");

  const navigate = useNavigate();

  return (
    <>
      <input
        type="text"
        name="token"
        id="token"
        value={token}
        onChange={e => setToken(e.currentTarget.value)}
      />
      <button
        onClick={() => {
          localStorage.setItem("token", token);
          navigate("/");
        }}
      >
        Set token
      </button>
    </>
  )
}