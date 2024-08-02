import { useEffect, useState } from "preact/hooks";
import mime from "mime";

import styles from "./list.module.scss"
import { ImagePreview } from "./preview";

type ResponseFile = {
  id: string;
  filename: string;
  bytes: number;
};

export function FileListPage() {
  const [page, setPage] = useState(1);
  const [files, setFiles] = useState<ResponseFile[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);

  const [preview, setPreview] = useState<string>();

  const totalPages = Math.max(1, Math.ceil(total / 10));

  useEffect(() => {
    fetchFiles();
  }, [page]);

  const fetchFiles = async () => {
    let token = localStorage.getItem("token");

    if(!token) return;

    setLoading(true);

    try {
      let response = await fetch(`${import.meta.env.VITE_BACKEND_URL}/manage/files?page=${page}`, {
        headers: {
          Authorization: token
        }
      });

      let result = await response.json();
      
      setTotal(result.total);
      setFiles(result.files);
    } finally {
      setLoading(false);
    }
  };

  const deleteFile = async (id: string) => {
    await fetch(`${import.meta.env.VITE_BACKEND_URL}/manage/files/${id}`, {
      method: "DELETE",
      headers: {
        Authorization: localStorage.getItem("token")
      } as any,
    });

    fetchFiles();
  };

  const renderIdWithPreview = (id: string) => {
    let type = mime.getType(id) || 'application/octet-stream';
    if(!type.startsWith("image")) {
      return <span
        onClick={() => navigator.clipboard.writeText(`${import.meta.env.VITE_BACKEND_URL}/files/${id}`)}
      >{id}</span>
    } else {
      return <ImagePreview id={id} />
    }
  }

  const renderFiles = () => {
    if(loading)
      return "Loading...";

    return (
      <>
      <div className={styles.files}>
        {
          files.map(f => (
            <div className={styles.file}>
              <div className={styles.side}>
                {renderIdWithPreview(f.id)}
                <span>{f.filename}</span>
              </div>
              <div className={[styles.side, styles.right].join(' ')}>
                <span>{f.bytes} bytes</span>
                <button
                  onClick={() => {
                    deleteFile(f.id);
                  }}
                >x</button>
              </div>
            </div>
          ))
        }
      </div>

      Total pages: {totalPages}
      </>
    )
  };

  const renderPreview = () => {
    if(!preview) return;

    return <img
      className={styles.preview}
      src={preview}
      onClick={() => setPreview(undefined)}
      style={{
        cursor: "pointer",
      }}
    />
  };

  return (
    <>
      Project F

      <div className={styles.pagination}>
        <button
          onClick={() => setPage(Math.max(1, page - 1))}
          disabled={page === 1}
        >&lt;</button>
        <span>{page}</span>
        <button
          onClick={() => setPage(Math.min(totalPages, page + 1))}
          disabled={page === totalPages} 
        >&gt;</button>
      </div>

      {renderFiles()}

      {renderPreview()}
    </>
  )
}