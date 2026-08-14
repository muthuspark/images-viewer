import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { ChevronRight, Folder, ImageOff, Minus, Plus, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

const MINIMUM_THUMBNAIL_SIZE = 140;
const THUMBNAIL_SIZE_STEP = 40;

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
  const [galleryWidth, setGalleryWidth] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [selectedImage, setSelectedImage] = useState<string | null>(null);
  const galleryViewportRef = useRef<HTMLDivElement>(null);
  const closeImageButtonRef = useRef<HTMLButtonElement>(null);
  const imageTriggerRef = useRef<HTMLButtonElement | null>(null);

  const maximumThumbnailSize = galleryWidth > 0
    ? Math.max(MINIMUM_THUMBNAIL_SIZE, galleryWidth - 8)
    : Number.POSITIVE_INFINITY;

  const selectFolder = useCallback(async (path: string) => {
    setSelectedImage(null);
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

  useEffect(() => {
    const viewport = galleryViewportRef.current;
    if (!viewport) return;

    const observer = new ResizeObserver(([entry]) => {
      setGalleryWidth(entry.contentRect.width);
    });

    observer.observe(viewport);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (galleryWidth === 0) return;
    setThumbnailSize((currentSize) => Math.min(currentSize, maximumThumbnailSize));
  }, [galleryWidth, maximumThumbnailSize]);

  useEffect(() => {
    if (!selectedImage) return;

    closeImageButtonRef.current?.focus();

    function handleViewerKeydown(event: KeyboardEvent) {
      if (event.key === "Escape") closeImage();
      if (event.key === "Tab") {
        event.preventDefault();
        closeImageButtonRef.current?.focus();
      }
    }

    window.addEventListener("keydown", handleViewerKeydown);
    return () => window.removeEventListener("keydown", handleViewerKeydown);
  }, [selectedImage]);

  function openImage(path: string, trigger: HTMLButtonElement) {
    imageTriggerRef.current = trigger;
    setSelectedImage(path);
  }

  function closeImage() {
    setSelectedImage(null);
    requestAnimationFrame(() => imageTriggerRef.current?.focus());
  }

  function adjustThumbnailSize(amount: number) {
    setThumbnailSize((currentSize) =>
      Math.min(
        maximumThumbnailSize,
        Math.max(MINIMUM_THUMBNAIL_SIZE, currentSize + amount),
      ),
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

      <main className="gallery" aria-busy={loading}>
        <div className="zoom-controls" role="group" aria-label="Image size">
          <button
            type="button"
            onClick={() => adjustThumbnailSize(-THUMBNAIL_SIZE_STEP)}
            aria-label="Decrease image size"
            title="Decrease image size"
            disabled={loading || images.length === 0 || thumbnailSize <= MINIMUM_THUMBNAIL_SIZE}
          >
            <Minus size={17} strokeWidth={1.8} aria-hidden="true" />
          </button>
          <button
            type="button"
            onClick={() => adjustThumbnailSize(THUMBNAIL_SIZE_STEP)}
            aria-label="Increase image size"
            title="Increase image size"
            disabled={loading || images.length === 0 || thumbnailSize >= maximumThumbnailSize}
          >
            <Plus size={17} strokeWidth={1.8} aria-hidden="true" />
          </button>
        </div>

        <div className="gallery-viewport" ref={galleryViewportRef}>
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
                <button
                  key={path}
                  className="image-thumbnail"
                  type="button"
                  aria-label="Open image full size"
                  onClick={(event) => openImage(path, event.currentTarget)}
                >
                  <img
                    src={convertFileSrc(path)}
                    alt=""
                    loading="lazy"
                    draggable={false}
                  />
                </button>
              ))}
            </div>
          ) : (
            <div className="empty-state">
              <ImageOff size={23} strokeWidth={1.35} aria-hidden="true" />
              <p>{error || "No images in this folder"}</p>
            </div>
          )}
        </div>
      </main>

      {selectedImage ? (
        <div className="image-viewer" role="dialog" aria-modal="true" aria-label="Full-size image">
          <img src={convertFileSrc(selectedImage)} alt="" draggable={false} />
          <button
            ref={closeImageButtonRef}
            className="image-viewer-close"
            type="button"
            onClick={closeImage}
            aria-label="Close full-size image"
            title="Close"
          >
            <X size={21} strokeWidth={1.8} aria-hidden="true" />
          </button>
        </div>
      ) : null}
    </div>
  );
}
