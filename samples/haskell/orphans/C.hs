module C where

import Data.Set
import A

instance Ord U where
  compare X X = EQ
  compare X Y = GT
  compare Y X = LT
  compare Y Y = EQ

-- ins' :: Ord U => U -> Set U -> Set U
ins' :: U -> Set U -> Set U
ins' = insert
