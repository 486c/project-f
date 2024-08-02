import { useEffect, useState } from "preact/hooks";

export function useWindowSize() {
  const [windowSize, setWindowSize] = useState<[number, number]>([0, 0]);

  useEffect(() => {
    const listener = () => {
      setWindowSize([window.innerHeight, window.innerHeight]);
    };

    window.addEventListener('resize', listener);

    return () => {
      window.removeEventListener('resize', listener);
    }
  }, []);

  return windowSize;
}