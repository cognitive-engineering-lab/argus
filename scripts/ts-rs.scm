(use-modules (ice-9 ftw)
             (ice-9 popen)
             (ice-9 rdelim)
             (ice-9 match)
             (srfi srfi-1))


;; add the bindings directory from each subdir of crates/
(define gen-dirs
  (let* ((rx (make-regexp "\\.{1,2}"))
         (argus-dir? (lambda (d) (not (regexp-exec rx d)))))
    (filter-map (lambda (d)
                  (and (argus-dir? d)
                       (let ((dir (format #f "crates/~a/bindings" d)))
                         (and (file-exists? dir) dir))))
                (scandir "crates"))))

;; Include files here if hand-written TS bindings are required.
(define ext-dirs '())

(define dest-dir "ide/packages/common/src")

(define (intersperse ls e)
  (let loop ((l ls))
    (match l
      ((f s t ...)
       (cons* f e (loop (cons s t))))
      (oth oth))))

(define (ts-file? file)
  (and (string? file)
       (string-suffix? ".ts" file)))

(define* (rm-suffix file #:optional s)
  (basename file (or s ".ts")))

(define (push base . subdirs)
  (apply string-append
         (intersperse (cons base subdirs) "/")))

(define (ensure-directory-exists directory-path)
  (or (file-exists? directory-path)
      (mkdir directory-path)))

(define (dir->tree dir)
  (remove-stat (file-system-tree dir)))

(define remove-stat
  (match-lambda
    ((name stat) name)
    ((name stat children ...)
     (list name (map remove-stat children)))))

(define (file->lines file)
  (call-with-input-file file
    (lambda (input-port)
      (let loop ((line (read-line input-port)))
        (if (eof-object? line)
            '()  ; End of file reached
            (cons line (loop (read-line input-port))))))))

(define (lines->file lines file)
  (call-with-output-file file
    (lambda (output-port)
      (format output-port "// I was auto-generated, please don't touch me!~%")
      (for-each (lambda (line)
                  (display line output-port)
                  (newline output-port))
                lines))))

(define (flatten-tree tree)
  (match tree
    ((_ contents)
     (apply append (map flatten-tree contents)))
    (s (if (ts-file? s)
           (list s)
           '()))))

(define (tree->lines cwd tree)
  (let loop ((cwd cwd) (tree tree))
    (match tree
      ((subdir contents)
       (let ((swd (push cwd subdir)))
         (apply append
                (map (lambda (i) (loop swd i)) contents))))
      (str
       (if (ts-file? str)
           (file->lines (push cwd str))
           '())))))

(define* (build-binding-interface source-dirs dest-dir #:optional extra-lines)
  (let* ((dir-trees (map dir->tree source-dirs))
         (all-lines (apply append (map (lambda (d t)
                                         (tree->lines (push d "..")
                                                      t))
                                       source-dirs dir-trees)))
         (filtered-lines (filter (lambda (line)
                                   (not (or (string-prefix? "import" line)
                                            (string-prefix? "//" line))))
                                 all-lines)))
    (lines->file (append (or extra-lines '()) filtered-lines)
                 (push dest-dir "bindings.ts"))))

(define (main)
  (build-binding-interface
   (append gen-dirs ext-dirs)
   dest-dir))

(main)
