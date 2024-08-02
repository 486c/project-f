import { useState } from "preact/hooks";
import { createPortal } from "preact/compat";

import styles from "./preview.module.scss";
// import { useWindowSize } from "../../hooks/windowSize";

type ImagePreviewProps = {
  id: string,
};

export function ImagePreview({ id }: ImagePreviewProps) {
  const [render, setRender] = useState(false);
  const [mouse, setMouse] = useState<[number, number]>([0, 0]);

  // TODO: offset portal to not go off screen
  // const windowSize = useWindowSize();

  function renderImagePortal() {
    if(!render) return null;

    return createPortal(
      (<div
        className={styles.preview}
        style={{
          top: mouse[1] + 10,
          left: mouse[0] + 10,
        }}
      >
        <img
          src={`${import.meta.env.VITE_BACKEND_URL}/files/${id}`}
        />
      </div>),
      document.querySelector("#portal")!
    );
  }

  return (
    <>
      {renderImagePortal()}

      <span
        onMouseEnter={() => {
          setRender(true); 
        }}
        onMouseMove={(e) => {
          setMouse([e.x, e.y]);
        }}
        onMouseLeave={() => {
          setRender(false);
        }}
        onClick={() => navigator.clipboard.writeText(`${import.meta.env.VITE_BACKEND_URL}/files/${id}`)}
        style={{ cursor: "pointer" }}
      >{id}</span>
    </>
  );
}