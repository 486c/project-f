import styles from "./app.module.scss";

import { UploadPage } from './pages/upload/upload';
import { FileListPage } from './pages/list/list';
import { RouterProvider, createHashRouter } from "react-router-dom";
import { SetTokenPage } from "./pages/token/token";

const router = createHashRouter([
  {
    path: "/",
    element: <UploadPage />,
  },
  {
    path: "/list",
    element: <FileListPage />,
  },
  {
    path: "/token",
    element: <SetTokenPage />,
  },
]);

export function App() {
  return (
    <div className={styles.container}>
      <RouterProvider router={router} />
    </div>
  )
}
