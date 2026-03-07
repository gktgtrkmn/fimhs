import Control.Concurrent (threadDelay)
import Control.Monad (forM, unless)
import Data.Map.Strict (Map)
import Data.Map.Strict qualified as Map
import Data.Time.Clock (UTCTime)
import System.Directory
  ( doesDirectoryExist,
    doesFileExist,
    getFileSize,
    getModificationTime,
    listDirectory,
  )
import System.Environment (getArgs)
import System.Exit (die)
import System.FilePath ((</>))

data FileMeta = FileMeta
  { size :: Integer,
    modified :: UTCTime
  }
  deriving (Show, Read, Eq)

type Snapshot = Map FilePath FileMeta

data Alert = Added | Modified | Deleted deriving (Show, Eq)

compareSnapshots :: Snapshot -> Snapshot -> Map FilePath Alert
compareSnapshots old new =
  let addedMap = Map.map (const Added) (Map.difference new old)
      deletedMap = Map.map (const Deleted) (Map.difference old new)
      both = Map.intersectionWith (,) old new
      changed = Map.filter (uncurry (/=)) both
      changedMap = Map.map (const Modified) changed
   in addedMap `Map.union` deletedMap `Map.union` changedMap

snapShotFile :: String
snapShotFile = ".fim_snapshot"

getRecursiveContents :: FilePath -> IO [FilePath]
getRecursiveContents topdir = do
  names <- listDirectory topdir
  let validNames = filter (/= snapShotFile) names
  paths <- forM validNames $ \name -> do
    let path = topdir </> name
    isDirectory <- doesDirectoryExist path
    if isDirectory
      then getRecursiveContents path
      else return [path]
  return (concat paths)

buildSnapshot :: FilePath -> IO Snapshot
buildSnapshot dir = do
  files <- getRecursiveContents dir
  metaList <- forM files $ \f -> do
    s <- getFileSize f
    t <- getModificationTime f
    return (f, FileMeta s t)
  return $ Map.fromList metaList

watchLoop :: FilePath -> Snapshot -> IO ()
watchLoop dir currentSnap = do
  threadDelay 3000000
  newSnap <- buildSnapshot dir
  let diff = compareSnapshots currentSnap newSnap
  unless (Map.null diff) $ do
    putStrLn "\n[!] Real Time Alert - Tampering Detected:"
    mapM_ (\(path, alert) -> putStrLn $ "  [" ++ show alert ++ "] " ++ path) (Map.toList diff)
    putStrLn "----------------"
    putStrLn "[*] Baseline updated. Resuming watch..."
  watchLoop dir newSnap

main :: IO ()
main = do
  args <- getArgs
  case args of
    ["init", dir] -> do
      putStrLn $ "[*] Building baseline snapshot for'" ++ dir ++ "'..."
      snap <- buildSnapshot dir
      writeFile snapShotFile (show snap)
      putStrLn $ "[+] Snapshot saved to " ++ snapShotFile
    ["check", dir] -> do
      exists <- doesFileExist snapShotFile
      if not exists
        then die "[-] Error: No snapshot found. Run 'init' first."
        else do
          putStrLn "[*] Scanning current directory..."
          oldData <- readFile snapShotFile
          let oldSnap = read oldData :: Snapshot
          newSnap <- buildSnapshot dir
          let diff = compareSnapshots oldSnap newSnap
          if Map.null diff
            then putStrLn "[*] Integrity check passed: No unauthorizated changes."
            else do
              putStrLn "[!] Alerts - Tampering Detected:"
              mapM_ (\(path, alert) -> putStrLn $ "   [" ++ show alert ++ "] " ++ path) (Map.toList diff)
    ["watch", dir] -> do
      putStrLn $ "[*] Starting watchin for '" ++ dir ++ "'..."
      putStrLn "[*] Press Ctrl-C to stop."
      initialSnap <- buildSnapshot dir
      watchLoop dir initialSnap
    _ -> die "Usage: runhaskell Fim.hs [init|check] <directory>"
