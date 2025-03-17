#!/usr/bin/env guile-wrapper.sh
!#

(use-modules (json)
             (srfi srfi-1)
             (web server)
             (web request)
             (web response)
             (web uri)
             (sxml simple)
             (ice-9 textual-ports)
             (ice-9 binary-ports)
             (ice-9 threads))



(define port (make-parameter 8080))
(define evaluation-results-fn
  (make-parameter "data/results-map.json"))

(define (find-map proc ls)
  (call/cc
   (lambda (return)
     (for-each (lambda (x)
                 (let ((result (proc x)))
                   (if result
                       (return result)))) ls) #f)))

(define (jref obj . fields)
  (fold (lambda (field obj) (assoc-ref obj field))
        obj
        fields))

(define (local-file fn)
  (format #f "http://localhost:~a~a" (port) fn))

(define (make-hidden-codebox body)
  `(div (@ (class "dropdown"))
    (div (@ (class "dropbtn"))
         (code "Click for full diagnostic"))
    (div (@ (class "dropdown-content"))
         (pre (code ,body)))))

(define (test-html name resultsv)
  (define (result-diagnostics r)
    (let ((diagnostics (jref r "diagnostics")))
      (or (and diagnostics (vector->list diagnostics))
          '())))
  (define results (vector->list resultsv))
  (define (file result)
    (let ((screenshotfn (jref result "argusScreenshotFn"))
          (fn (jref result "filename"))
          (rustc-msgs (map (lambda (d) (jref d "message" "message"))
                           (result-diagnostics result))))
      `(div (@ (style "padding: 0.5em; border: 1px dashed black;"))
        (h2 ,fn)
        (div (@ (style "max-height: 25em;overflow-y: scroll;"))
             (img (@ (src ,(local-file screenshotfn)))))
        "Principal message: "
        (pre (code ,(string-join rustc-msgs "\n")))
        ,(make-hidden-codebox
          (find-map (lambda (d) (jref d "message" "rendered"))
                    (result-diagnostics result))))))
  `(div (h1 ,name)
    ,@(map file results)))

(define page-style "
code { font-family: monospace;  }
pre code {
  display: block;
  padding: 0.5em;
  background-color: #f0f0f0;
  border: 1px solid #ccc;
  border-radius: 5px;
}

.dropdown-content {
  display: none;
}

.dropdown:hover {
  font-weight: bold;
  shadow: 1px 1px 1px #000;
}")

(define page-script "
window.addEventListener('load', function(event) {
  document.querySelectorAll('.dropbtn').forEach(function(elem) {
    elem.addEventListener('click', function() {
      var content = this.nextElementSibling;
      if (content.style.display === 'block') {
        content.style.display = 'none';
      } else {
        content.style.display = 'block';
      }
    })
  });
});")

(define (from-file fn)
  (define all-tests
    (map (lambda (o)
           (let ((name (jref o "test"))
                 (results (jref o "result")))
             (test-html name results)))
         (vector->list (json->scm (open-file fn "r")))))
  `(html (head (title "Argus Test Evaluation"))
    (style ,page-style)
    (script ,page-script)
    (body ,@all-tests)))

(define (request-path-components request)
  (split-and-decode-uri-path (uri-path (request-uri request))))

(define (response-404)
  (values (build-response
           #:code 404
           #:headers `((content-type . (text/plain))))
          "404 Not Found"))

(define (handler request body)
  (cond
   ((equal? (request-path-components request) '("eval"))
    (values '((content-type . (text/html)))
            (let ((contents (from-file (evaluation-results-fn))))
              (with-output-to-string
                (lambda ()
                  (sxml->xml contents))))))
   ((file-exists? (uri-path (request-uri request)))
    (let ((fn (uri-path (request-uri request))))
      (values '((content-type . (image/png)))
              (get-bytevector-all (open-file fn "rb")))))
   (else (response-404))))

(let ((t (make-thread (lambda () (run-server handler)))))
  (format #t "[serving on localhost]...~%enter anything to cancel: " (port))
  (get-char (current-input-port))
  (cancel-thread t))
