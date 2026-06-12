import { useEffect, useMemo, useRef, useState } from "react";
import type { PDFDocumentProxy, PDFPageProxy } from "pdfjs-dist/types/src/pdf";

export type FilePreviewViewModel = {
  assetUrl?: string;
  content?: string;
  extension: string;
  name: string;
  path: string;
  previewError?: string;
  type: "image" | "pdf" | "text" | "word-legacy" | "external";
};

type FilePreviewViewerProps = {
  file: FilePreviewViewModel;
};

type PdfTextPage = {
  pageNumber: number;
  text: string;
};

type PdfSearchMatch = {
  pageNumber: number;
};

type PdfRenderTask = ReturnType<PDFPageProxy["render"]>;

const PDF_SCALE_OPTIONS = [0.75, 1, 1.25, 1.5, 2, 2.5];
const IMAGE_SCALE_OPTIONS = [0.5, 0.75, 1, 1.5, 2, 3];
let pdfjsLoadPromise: Promise<typeof import("pdfjs-dist")> | null = null;

export function FilePreviewViewer({ file }: FilePreviewViewerProps) {
  const [textSearch, setTextSearch] = useState("");
  const [activeTextMatch, setActiveTextMatch] = useState(0);
  const textMatches = useMemo(() => collectTextMatches(file.content ?? "", textSearch), [file.content, textSearch]);

  useEffect(() => {
    setTextSearch("");
    setActiveTextMatch(0);
  }, [file.path]);

  useEffect(() => {
    setActiveTextMatch(0);
  }, [textSearch]);

  const goToTextMatch = (direction: 1 | -1) => {
    if (!textMatches.length) return;
    setActiveTextMatch((current) => (current + direction + textMatches.length) % textMatches.length);
  };

  return (
    <div className="file-preview" aria-label="只读文件预览">
      {file.type === "image" ? (
        <ImagePreview file={file} />
      ) : file.type === "pdf" ? (
        <PdfPreview file={file} />
      ) : file.type === "text" ? (
        <TextPreview
          content={file.content ?? ""}
          search={textSearch}
          activeMatch={activeTextMatch}
          matchCount={textMatches.length}
          onSearchChange={setTextSearch}
          onPrevious={() => goToTextMatch(-1)}
          onNext={() => goToTextMatch(1)}
        />
      ) : file.type === "word-legacy" ? (
        <ExternalPreview title="Word 二进制格式" description="DOCX 可直接编辑保存；DOC/WPS 需要本机转换引擎后才能在 Wridian 内安全编辑。" />
      ) : (
        <ExternalPreview title="只读文件" description="当前格式不能在文件编辑区直接编辑。请用本机程序查看。" />
      )}
    </div>
  );
}

function ImagePreview({ file }: { file: FilePreviewViewModel }) {
  const [scale, setScale] = useState(1);
  const [fitToPane, setFitToPane] = useState(true);

  useEffect(() => {
    setScale(1);
    setFitToPane(true);
  }, [file.path]);

  if (!file.assetUrl) {
    return <PreviewError message={file.previewError || "无法读取这张图片。"} />;
  }

  const zoom = (direction: 1 | -1) => {
    setFitToPane(false);
    setScale((current) => stepScale(current, IMAGE_SCALE_OPTIONS, direction));
  };

  return (
    <div className="file-preview-canvas image">
      <div className="viewer-titlebar" title={file.name}>{stripFileExtension(file.name)}</div>
      <div className="viewer-toolbar" aria-label="图片预览工具">
        <div className="viewer-toolbar-group">
          <button type="button" className="viewer-tool-button" onClick={() => zoom(-1)} aria-label="缩小图片">
            -
          </button>
          <button type="button" className="viewer-tool-button" onClick={() => zoom(1)} aria-label="放大图片">
            +
          </button>
          <button type="button" className="viewer-tool-button text" onClick={() => setFitToPane(true)}>
            适应
          </button>
          <button
            type="button"
            className="viewer-tool-button text"
            onClick={() => {
              setFitToPane(false);
              setScale(1);
            }}
          >
            100%
          </button>
        </div>
        <div className="viewer-toolbar-status">{fitToPane ? "适应窗口" : `${Math.round(scale * 100)}%`}</div>
      </div>
      <div className={`image-preview-stage ${fitToPane ? "fit" : ""}`}>
        <img
          src={file.assetUrl}
          alt={file.name}
          style={fitToPane ? undefined : { width: `${scale * 100}%`, maxWidth: "none" }}
        />
      </div>
    </div>
  );
}

function PdfPreview({ file }: { file: FilePreviewViewModel }) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const viewerRef = useRef<HTMLDivElement | null>(null);
  const renderSeqRef = useRef(0);
  const renderTaskRef = useRef<PdfRenderTask | null>(null);
  const [document, setDocument] = useState<PDFDocumentProxy | null>(null);
  const [pageNumber, setPageNumber] = useState(1);
  const [pageCount, setPageCount] = useState(0);
  const [scale, setScale] = useState(1);
  const [loading, setLoading] = useState(false);
  const [rendering, setRendering] = useState(false);
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");
  const [textPages, setTextPages] = useState<PdfTextPage[]>([]);
  const [textLoading, setTextLoading] = useState(false);
  const [activeMatch, setActiveMatch] = useState(0);

  useEffect(() => {
    let cancelled = false;
    let loadedDocument: PDFDocumentProxy | null = null;
    setDocument(null);
    setPageNumber(1);
    setPageCount(0);
    setScale(1);
    setError("");
    setSearch("");
    setTextPages([]);
    setActiveMatch(0);

    if (!file.assetUrl) {
      setError(file.previewError || "无法读取这个 PDF。");
      return;
    }

    setLoading(true);
    let task: ReturnType<typeof import("pdfjs-dist").getDocument> | null = null;
    void dataUrlToBytes(file.assetUrl)
      .then((data) => loadPdfjs().then((pdfjsLib) => ({ data, pdfjsLib })))
      .then(({ data, pdfjsLib }) => {
        if (cancelled) return null;
        task = pdfjsLib.getDocument({ data });
        return task.promise;
      })
      .then((loaded) => {
        if (!loaded) return;
        if (cancelled) {
          loaded.cleanup();
          return;
        }
        loadedDocument = loaded;
        setDocument(loaded);
        setPageCount(loaded.numPages);
      })
      .catch((loadError: unknown) => {
        if (!cancelled) setError(loadError instanceof Error ? loadError.message : String(loadError));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
      void task?.destroy();
      loadedDocument?.cleanup();
    };
  }, [file.assetUrl, file.path, file.previewError]);

  useEffect(() => {
    if (!document) return;
    let cancelled = false;
    setTextPages([]);
    setTextLoading(true);

    void extractPdfText(document)
      .then((pages) => {
        if (!cancelled) setTextPages(pages);
      })
      .catch(() => {
        if (!cancelled) setTextPages([]);
      })
      .finally(() => {
        if (!cancelled) setTextLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [document]);

  useEffect(() => {
    if (!document) return;
    let cancelled = false;
    const seq = renderSeqRef.current + 1;
    renderSeqRef.current = seq;
    renderTaskRef.current?.cancel();
    renderTaskRef.current = null;
    setRendering(true);
    setError("");

    void renderPdfPage(document, pageNumber, scale, canvasRef.current, (task) => {
      renderTaskRef.current = task;
    })
      .catch((renderError: unknown) => {
        if (isPdfRenderCancelled(renderError)) return;
        if (!cancelled && renderSeqRef.current === seq) {
          setError(renderError instanceof Error ? renderError.message : String(renderError));
        }
      })
      .finally(() => {
        if (!cancelled && renderSeqRef.current === seq) setRendering(false);
      });

    return () => {
      cancelled = true;
      renderTaskRef.current?.cancel();
    };
  }, [document, pageNumber, scale]);

  const matches = useMemo(() => collectPdfMatches(textPages, search), [textPages, search]);

  useEffect(() => {
    setActiveMatch(0);
  }, [search]);

  useEffect(() => {
    const match = matches[activeMatch];
    if (match && match.pageNumber !== pageNumber) {
      setPageNumber(match.pageNumber);
    }
  }, [activeMatch, matches, pageNumber]);

  const goToMatch = (direction: 1 | -1) => {
    if (!matches.length) return;
    setActiveMatch((current) => (current + direction + matches.length) % matches.length);
  };

  const goToPage = (nextPage: number) => {
    setPageNumber(clampPage(nextPage, pageCount || 1));
    viewerRef.current?.scrollTo({ top: 0, behavior: "smooth" });
  };

  const zoom = (direction: 1 | -1) => {
    setScale((current) => stepScale(current, PDF_SCALE_OPTIONS, direction));
  };

  return (
    <div className="file-preview-canvas document pdf-viewer">
      <div className="viewer-titlebar" title={file.name}>{stripFileExtension(file.name)}</div>
      <div className="viewer-toolbar" aria-label="PDF 预览工具">
        <div className="viewer-toolbar-group">
          <button type="button" className="viewer-tool-button" onClick={() => goToPage(pageNumber - 1)} disabled={pageNumber <= 1} aria-label="上一页">
            ‹
          </button>
          <label className="viewer-page-control">
            <input
              value={pageNumber}
              onChange={(event) => goToPage(Number(event.target.value) || 1)}
              inputMode="numeric"
              aria-label="PDF 页码"
            />
            <span>/ {pageCount || "-"}</span>
          </label>
          <button type="button" className="viewer-tool-button" onClick={() => goToPage(pageNumber + 1)} disabled={!pageCount || pageNumber >= pageCount} aria-label="下一页">
            ›
          </button>
        </div>
        <div className="viewer-toolbar-group">
          <button type="button" className="viewer-tool-button" onClick={() => zoom(-1)} aria-label="缩小 PDF">
            -
          </button>
          <button type="button" className="viewer-tool-button" onClick={() => zoom(1)} aria-label="放大 PDF">
            +
          </button>
          <span className="viewer-toolbar-status">{Math.round(scale * 100)}%</span>
        </div>
        <div className="viewer-search">
          <input
            type="search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="搜索 PDF"
            aria-label="搜索 PDF"
          />
          <span className="viewer-search-count">{search ? `${matches.length ? activeMatch + 1 : 0}/${matches.length}` : textLoading ? "索引中" : ""}</span>
          <button type="button" className="viewer-tool-button" onClick={() => goToMatch(-1)} disabled={!matches.length} aria-label="上一个搜索结果">
            ↑
          </button>
          <button type="button" className="viewer-tool-button" onClick={() => goToMatch(1)} disabled={!matches.length} aria-label="下一个搜索结果">
            ↓
          </button>
        </div>
      </div>
      <div className="pdf-viewer-stage" ref={viewerRef}>
        {loading ? <div className="file-preview-placeholder">正在加载 PDF...</div> : null}
        {error ? <PreviewError message={error} /> : null}
        <canvas ref={canvasRef} className={rendering ? "is-rendering" : ""} aria-label={`${file.name} 第 ${pageNumber} 页`} />
      </div>
    </div>
  );
}

function TextPreview({
  activeMatch,
  content,
  matchCount,
  onNext,
  onPrevious,
  onSearchChange,
  search,
}: {
  activeMatch: number;
  content: string;
  matchCount: number;
  onNext: () => void;
  onPrevious: () => void;
  onSearchChange: (value: string) => void;
  search: string;
}) {
  return (
    <div className="file-preview-canvas text">
      <div className="viewer-toolbar" aria-label="文本查看工具">
        <div className="viewer-search">
          <input
            type="search"
            value={search}
            onChange={(event) => onSearchChange(event.target.value)}
            placeholder="搜索文本"
            aria-label="搜索文本"
          />
          <span className="viewer-search-count">{search ? `${matchCount ? activeMatch + 1 : 0}/${matchCount}` : ""}</span>
          <button type="button" className="viewer-tool-button" onClick={onPrevious} disabled={!matchCount} aria-label="上一个搜索结果">
            ↑
          </button>
          <button type="button" className="viewer-tool-button" onClick={onNext} disabled={!matchCount} aria-label="下一个搜索结果">
            ↓
          </button>
        </div>
      </div>
      <pre>{renderHighlightedText(content, search, activeMatch)}</pre>
    </div>
  );
}

function ExternalPreview({ description, title }: { description: string; title: string }) {
  return (
    <div className="file-preview-canvas external">
      <div className="file-preview-placeholder">
        <strong>{title}</strong>
        <span>{description}</span>
      </div>
    </div>
  );
}

function PreviewError({ message }: { message: string }) {
  return (
    <div className="file-preview-canvas external">
      <div className="file-preview-placeholder">
        <strong>预览加载失败</strong>
        <span>{message}</span>
      </div>
    </div>
  );
}

function stepScale(current: number, options: number[], direction: 1 | -1) {
  const sorted = [...options].sort((a, b) => a - b);
  if (direction > 0) {
    return sorted.find((value) => value > current + 0.001) ?? sorted[sorted.length - 1];
  }
  return [...sorted].reverse().find((value) => value < current - 0.001) ?? sorted[0];
}

function clampPage(page: number, pageCount: number) {
  return Math.max(1, Math.min(pageCount, page));
}

async function renderPdfPage(
  document: PDFDocumentProxy,
  pageNumber: number,
  scale: number,
  canvas: HTMLCanvasElement | null,
  onRenderTask: (task: PdfRenderTask) => void,
) {
  if (!canvas) return;
  const page = await document.getPage(pageNumber);
  const viewport = page.getViewport({ scale });
  const context = canvas.getContext("2d");
  if (!context) throw new Error("无法创建 PDF 渲染画布。");
  const outputScale = window.devicePixelRatio || 1;
  canvas.width = Math.floor(viewport.width * outputScale);
  canvas.height = Math.floor(viewport.height * outputScale);
  canvas.style.width = `${Math.floor(viewport.width)}px`;
  canvas.style.height = `${Math.floor(viewport.height)}px`;
  context.setTransform(outputScale, 0, 0, outputScale, 0, 0);
  const renderTask = page.render({ canvas, canvasContext: context, viewport });
  onRenderTask(renderTask);
  try {
    await renderTask.promise;
  } finally {
    page.cleanup();
  }
}

function isPdfRenderCancelled(error: unknown) {
  return error instanceof Error && error.name === "RenderingCancelledException";
}

async function extractPdfText(document: PDFDocumentProxy): Promise<PdfTextPage[]> {
  const pages: PdfTextPage[] = [];
  for (let pageNumber = 1; pageNumber <= document.numPages; pageNumber += 1) {
    const page: PDFPageProxy = await document.getPage(pageNumber);
    const textContent = await page.getTextContent();
    const text = textContent.items
      .map((item) => ("str" in item && typeof item.str === "string" ? item.str : ""))
      .join(" ");
    pages.push({ pageNumber, text });
    page.cleanup();
  }
  return pages;
}

function loadPdfjs() {
  pdfjsLoadPromise ??= Promise.all([
    import("pdfjs-dist"),
    import("pdfjs-dist/build/pdf.worker.min.mjs?url"),
  ]).then(([pdfjsLib, worker]) => {
    pdfjsLib.GlobalWorkerOptions.workerSrc = worker.default;
    return pdfjsLib;
  });
  return pdfjsLoadPromise;
}

async function dataUrlToBytes(dataUrl: string) {
  const commaIndex = dataUrl.indexOf(",");
  if (!dataUrl.startsWith("data:") || commaIndex < 0) {
    throw new Error("PDF 预览数据格式无效。");
  }
  const metadata = dataUrl.slice(0, commaIndex);
  const payload = dataUrl.slice(commaIndex + 1);
  if (!metadata.includes(";base64")) {
    const decoded = decodeURIComponent(payload);
    return new TextEncoder().encode(decoded);
  }
  const binary = window.atob(payload);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function stripFileExtension(name: string) {
  return name.replace(/\.[^.\\/]+$/, "");
}

function collectPdfMatches(pages: PdfTextPage[], query: string): PdfSearchMatch[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [];
  return pages.flatMap((page) => collectTextMatches(page.text, needle).map(() => ({
    pageNumber: page.pageNumber,
  })));
}

function collectTextMatches(content: string, query: string) {
  const needle = query.trim().toLowerCase();
  if (!needle) return [];
  const haystack = content.toLowerCase();
  const matches: number[] = [];
  let index = haystack.indexOf(needle);
  while (index >= 0) {
    matches.push(index);
    index = haystack.indexOf(needle, index + Math.max(needle.length, 1));
  }
  return matches;
}

function renderHighlightedText(content: string, query: string, activeMatch: number) {
  const needle = query.trim();
  const matches = collectTextMatches(content, needle);
  if (!needle || !matches.length) return content;
  const pieces: React.ReactNode[] = [];
  let cursor = 0;
  matches.forEach((matchIndex, index) => {
    if (matchIndex > cursor) pieces.push(content.slice(cursor, matchIndex));
    pieces.push(
      <mark key={`${matchIndex}-${index}`} className={index === activeMatch ? "is-active" : ""}>
        {content.slice(matchIndex, matchIndex + needle.length)}
      </mark>,
    );
    cursor = matchIndex + needle.length;
  });
  if (cursor < content.length) pieces.push(content.slice(cursor));
  return pieces;
}
