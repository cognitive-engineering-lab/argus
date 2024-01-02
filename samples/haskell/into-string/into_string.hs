-- IntoString example

import Data.List

class IntoString a where
    intoString :: a -> String

instance IntoString (Int, Int) where
    intoString (a, b) = "(" ++ (show a) ++ ", " ++ (show b) ++ ")"

instance (IntoString a) => IntoString [a] where
    intoString = (\s -> "[" ++ s ++ "]") . concat .  intersperse ", " .  map intoString

printIt :: IntoString a => a -> IO ()
printIt = putStrLn . intoString

main :: IO ()
main = do
  -- NOTE: without type annotations this fails horribly
  -- printIt [(1, 2), (3, 4)]
  -- printIt ([(1, 2), (3, 4)] :: [(Int, Int)])
  printIt ([(1, 2.0), (3, 4.0)] :: [(Int, Float)])
  return ()
