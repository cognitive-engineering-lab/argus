module Main where

import Data.Set
import A
import B
import C

-- Because each respective Ord class instances 
-- were resolved locally, there is no error here.
--
-- The following signatures already resolved class instances.
--   B.ins  :: U -> Set U -> Set U
--   C.ins' :: U -> Set U -> Set U
--
-- Do not require an outside module to provide 
-- an instance of the Ord class for U. IFF the 
-- signatures are changed to:
--
--   B.ins  :: Ord U => U -> Set U -> Set U
--   C.ins' :: Ord U => U -> Set U -> Set U
--
-- Then there is a compilation error for overlapping instances.
--
-- D.hs:9:8: error:
--   • Overlapping instances for Ord U arising from a use of ‘ins'’
--     Matching instances:
--       instance [safe] Ord U -- Defined at C.hs:6:10
--       instance [safe] Ord U -- Defined at B.hs:6:10
--   • In the first argument of ‘($)’, namely ‘ins' X’
--     In the expression: ins' X $ ins X $ ins Y $ empty
--     In an equation for ‘test’: test = ins' X $ ins X $ ins Y $ empty

test :: Set U
test = ins' X $ ins X $ ins Y $ empty

main :: IO ()
main = print test
