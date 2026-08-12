import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { ChevronRight, Folder, ImageOff } from "lucide-react";
import { useCallback, useEffect, useState } from "react";

type FolderEntry = {
  name: string;
  path: string;
};

type DirectoryContents = {
  folders: FolderEntry[];
  images: string[];
};

type TreeNodeProps = {
  folder: FolderEntry;
  depth: number;
  selectedPath: string;
  onSelect: (path: string) => void;
  initiallyOpen?: boolean;
};

function TreeNode({
  folder,
  depth,
  selectedPath,
  onSelect,
  initiallyOpen = false,
}: TreeNodeProps) {
  const [open, setOpen] = useState(initiallyOpen);
  const [children, setChildren] = useState<FolderEntry[] | null>(null);
  const [unavailable, setUnavailable] = useState(false);

  const loadChildren = useCallback(async () => {
    if (children !== null || unavailable) return;

    try {
      const contents = await invoke<DirectoryContents>("list_directory", {
        path: folder.path,
      });
      setChildren(contents.folders);
    } catch {
      setUnavailable(true);
    }
  }, [children, folder.path, unavailable]);

  useEffect(() => {
    if (initiallyOpen) void loadChildren();
  }, [initiallyOpen, loadChildren]);

  function toggle(event: React.MouseEvent) {
    event.stopPropagation();
    const nextOpen = !open;
    setOpen(nextOpen);
    if (nextOpen) void loadChildren();
  }

  return (
    <li>
      <div
        className={`tree-row ${selectedPath === folder.path ? "is-selected" : ""}`}
        style={{ paddingInlineStart: `${10 + depth * 16}px` }}
      >
        <button
          className="tree-toggle"
          type="button"
          onClick={toggle}
          aria-label={`${open ? "Collapse" : "Expand"} ${folder.name}`}
          aria-expanded={open}
          disabled={unavailable}
        >
          <ChevronRight className={open ? "is-open" : ""} size={14} strokeWidth={1.8} />
        </button>
        <button className="tree-label" type="button" onClick={() => onSelect(folder.path)}>
          <Folder size={15} strokeWidth={1.6} aria-hidden="true" />
          <span>{folder.name}</span>
        </button>
      </div>

      {open && children && children.length > 0 ? (
        <ul>
          {children.map((child) => (
            <TreeNode
              key={child.path}
              folder={child}
              depth={depth + 1}
              selectedPath={selectedPath}
              onSelect={onSelect}
            />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

export default function App() {
  const [roots, setRoots] = useState<FolderEntry[]>([]);
  const [selectedPath, setSelectedPath] = useState("");
  const [images, setImages] = useState<string[]>([]);
  const [thumbnailSize, setThumbnailSize] = useState(190);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const selectFolder = useCallback(async (path: string) => {
    setSelectedPath(path);
    setLoading(true);
    setError("");

    try {
      const contents = await invoke<DirectoryContents>("list_directory", { path });
      setImages(contents.images);
    } catch (reason) {
      setImages([]);
      setError(typeof reason === "string" ? reason : "This folder cannot be opened.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    async function initialize() {
      try {
        const systemRoots = await invoke<FolderEntry[]>("list_roots");
        setRoots(systemRoots);
        if (systemRoots[0]) await selectFolder(systemRoots[0].path);
      } catch {
        setError("The filesystem is unavailable.");
        setLoading(false);
      }
    }

    void initialize();
  }, [selectFolder]);

  function resizeImages(event: React.WheelEvent<HTMLElement>) {
    if (loading || images.length === 0 || event.deltaY === 0) return;

    const minimumSize = 140;
    const maximumSize = Math.max(minimumSize, event.currentTarget.clientWidth - 8);
    const direction = event.deltaY > 0 ? 1 : -1;
    const step = Math.min(64, Math.max(18, Math.abs(event.deltaY) * 0.25));

    setThumbnailSize((currentSize) =>
      Math.min(maximumSize, Math.max(minimumSize, currentSize + direction * step)),
    );
  }

  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="Filesystem navigation">
        <div className="sidebar-title">Folders</div>
        <nav>
          <ul className="tree-root">
            {roots.map((root) => (
              <TreeNode
                key={root.path}
                folder={root}
                depth={0}
                selectedPath={selectedPath}
                onSelect={selectFolder}
                initiallyOpen
              />
            ))}
          </ul>
        </nav>
      </aside>

      <main className="gallery" aria-busy={loading} onWheel={resizeImages}>
        {loading ? (
          <div
            className="gallery-grid is-loading"
            style={{ "--thumbnail-size": `${thumbnailSize}px` } as React.CSSProperties}
            aria-label="Loading images"
          >
            {Array.from({ length: 12 }, (_, index) => (
              <div className="image-skeleton" key={index} />
            ))}
          </div>
        ) : images.length > 0 ? (
          <div
            className="gallery-grid"
            style={{ "--thumbnail-size": `${thumbnailSize}px` } as React.CSSProperties}
          >
            {images.map((path) => (
              <img
                key={path}
                src={convertFileSrc(path)}
                alt=""
                loading="lazy"
                draggable={false}
              />
            ))}
          </div>
        ) : (
          <div className="empty-state">
            <ImageOff size={23} strokeWidth={1.35} aria-hidden="true" />
            <p>{error || "No images in this folder"}</p>
          </div>
        )}
      </main>
    </div>
  );
}
