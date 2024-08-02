import { useEffect, useRef, useState } from 'preact/hooks'
import styles from './upload.module.scss'
import { useNavigate } from 'react-router-dom';
import axios from 'axios';

type CurrentStatusLoading = {
  status: 'loading',
  progress: number,
};

type CurrentStatusSuccess = {
  status: 'success',
  existed: boolean,
  data: string,
};

type CurrentStatusError = {
  status: 'error',
  data: string,
};

type CurrentStatus = 
  CurrentStatusLoading |
  CurrentStatusSuccess |
  CurrentStatusError;

export function UploadPage() {
  const [filename, setFilename] = useState<string>();
  const [imgSrc, setImgSrc] = useState<string>();
  const [status, setStatus] = useState<CurrentStatus>();
  const [dragFile, setDragFile] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);

  const navigate = useNavigate();

  const onChange = () => {
    const file = inputRef.current?.files?.[0];

    if(!file) return;

    setFilename(file.name);

    if(!file.type.startsWith("image")) {
      setImgSrc("");
      return;
    }

    setImgSrc(URL.createObjectURL(file));
  };

  const beginChunkUpload = async (token: string, file: File): Promise<string> => {
    let filename = file.name;

    let { data } = await axios.post(`${import.meta.env.VITE_BACKEND_URL}/manage/upload/begin_chunks`, {
      filename,
    }, {
      headers: {
        Authorization: token,
        "Content-Range": file.size,
      },
    });

    return data.id;
  };

  const finishChunkUpload = async (token: string, id: string): Promise<{ id: string, existed: boolean }> => {
    let { data } = await axios.post(`${import.meta.env.VITE_BACKEND_URL}/manage/upload/end_chunks`, {
      id
    }, {
      headers: {
        Authorization: token,
      },
    });

    return data;
  }

  const uploadLargeFile = async (token: string, file: File) => {
    let uploadId = await beginChunkUpload(token, file);

    const CHUNK_SIZE = 1024 * 1024 * 50;

    let chunks: [number, Blob][] = [];
    let progress: number[] = [];

    for(let i = 0; i < file.size; i += CHUNK_SIZE) {
      chunks.push([i, file.slice(i, i + CHUNK_SIZE)]);
      progress.push(0);
    }

    console.log(chunks);

    let uploads = chunks.map((chunk, i) => {
      let body = new FormData();
      body.set("chunk", chunk[1]);
      return axios.post(`${import.meta.env.VITE_BACKEND_URL}/manage/upload/chunk/${uploadId}`, body, {
        headers: {
          Authorization: token,
          "Content-Range": chunk[0],
        },
        onUploadProgress: (e) => setStatus((status) => {
          let chunkProgress = Math.round((e.loaded) * 100 / (e.total ?? e.loaded));

          progress[i] = chunkProgress;
          let totalProgress = progress.reduce((a, b) => a + b, 0) / progress.length;

          if(status?.status == 'loading') {
            return {
              status: 'loading',
              progress: totalProgress,
            };
          } else {
            return status;
          }
        }),
      });
    });

    try {
      await Promise.all(uploads);

      let result = await finishChunkUpload(token, uploadId);

      let url = `${import.meta.env.VITE_BACKEND_URL}/files/${result.id}`;

      setStatus({
        status: "success",
        existed: result.existed,
        data: url,
      })
    } catch(err: any) {
      await axios.post(`${import.meta.env.VITE_BACKEND_URL}/manage/upload/discard/${uploadId}`, {}, {
        headers: {
          Authorization: token,
        },
      });

      setStatus({
        status: "error",
        data: err.response.data,
      });
    }
  };

  const uploadFile = async () => {
    let token = localStorage.getItem("token");

    if(!token) {
      setStatus({
        status: "error",
        data: "piss off"
      });
      return;
    }

    const file = inputRef.current?.files?.[0];
    if(!file) return;

    setStatus({
      status: "loading",
      progress: 0,
    });

    console.log(file.size);

    if(file.size > 1024 * 1024 * 80) {
      return uploadLargeFile(token, file);
    }

    let body = new FormData();
    body.append("file", file);

    try {
      let { data } = await axios.post(
        `${import.meta.env.VITE_BACKEND_URL}/manage/upload/file`,
        body,
        {
          headers: {
            Authorization: token,
          },
          onUploadProgress: (e) => setStatus((status) => {
            let progress = Math.round((e.loaded * 100) / (e.total ?? e.loaded));

            if(status?.status == 'loading') {
              return {
                status: 'loading',
                progress,
              };
            } else {
              return status;
            }
          }),
        }
      );

      let url = `${import.meta.env.VITE_BACKEND_URL}/files/${data.id}`;

      setStatus({
        status: "success",
        existed: data.existed,
        data: url
      });
    } catch(err: any) {
      setStatus({
        status: "error",
        data: err.response.data
      });
    }
  };

  const renderLink = (link: string) => (
    <code
      className={styles.code}
      onClick={() => {
        navigator.clipboard.writeText(link);
      }}
    >
      {link}
    </code>
  );

  const renderStatus = () => {
    if(!status) return;

    let inner;

    if(status.status === "error") {
      inner = `Error: ${status.data}`;
    } else if(status.status === "loading") {
      inner = (
        <>
          <div
            className={styles.progress}
            style={{
              width: `${status.progress}%`,
            }}
          />
          <div className={styles.loader} />
        </>
      );
    } else if(status.status === "success") {
      if(status.existed) {
        inner = (<>Old file found: {renderLink(status.data!)}</>)
      } else {
        inner = (<>Your file link: {renderLink(status.data!)}</>);
      }
    }

    return (
      <div className={[styles.status, styles[status.status]].join(' ')}>
        {inner}
      </div>
    )
  };

  useEffect(() => {
    let pasteListener = (ev: ClipboardEvent) => {
      let file = ev.clipboardData?.files.item(0);
      if(!file) return;
      if(!file.type.startsWith("image")) return;
      inputRef.current!.files = ev.clipboardData!.files;
      inputRef.current!.dispatchEvent(new Event('change'));
    };

    let dndListener = (ev: DragEvent) => {
      ev.preventDefault();

      setDragFile(false);

      let { files } = ev.dataTransfer!;

      inputRef.current!.files = files;
      inputRef.current!.dispatchEvent(new Event('change'));
    };

    let dragover = (ev: DragEvent) => {
      ev.preventDefault();
      setDragFile(true);
    };

    document.addEventListener('paste', pasteListener);
    document.addEventListener('dragover', dragover);
    document.addEventListener('drop', dndListener);

    return () => {
      document.removeEventListener('paste', pasteListener);
      document.removeEventListener('dragover', dragover);
      document.removeEventListener('drop', dndListener);
    };
  }, []);

  return (
    <>
      Project F

      <label htmlFor="file">
        <input
          type="file"
          name="file"
          id="file"
          ref={inputRef}
          onChange={onChange}
          style={{ display: "none" }}
        />
        <div className={styles["file-label"]}>
          {dragFile ? "nom" : "Drop file here"}
        </div>
      </label>
      <span>{filename}</span>
      <img className={styles.img} src={imgSrc}></img>

      <button
        className={styles["upload-button"]}
        onClick={uploadFile}
        disabled={status?.status === "loading"}
      >Upload</button>

      {renderStatus()}

      <div className={styles.navigation}>
        <button
          onClick={() => {
            navigate("/list");
          }}
          className={styles["upload-button"]}
        >
          Files
        </button>
        <button
          onClick={() => {
            navigate("/token");
          }}
          className={styles["upload-button"]}
        >
          Token
        </button>
      </div>
    </>
  )
}
